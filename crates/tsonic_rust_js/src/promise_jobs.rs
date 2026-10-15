use std::cell::{Cell, OnceCell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::task::Waker;
use std::thread::{self, ThreadId};

use async_task::{Runnable, Task};
use futures_util::FutureExt;
use slab::Slab;
use tsonic_rust_runtime::dispatch::DispatchOwner;
use tsonic_rust_runtime::JsError;

const MAX_PENDING_JOBS: usize = 1 << 20;
const MAX_JOBS_PER_TURN: usize = 1024;
const NO_LOOP: &str = "No JavaScript event loop is running on this thread.";
const CLOSING: usize = 1 << (usize::BITS - 1);

type ReadyJob = Runnable<usize>;

struct IngressState {
    ready: VecDeque<ReadyJob>,
    closing: bool,
    closed: bool,
}

struct Ingress {
    owner: DispatchOwner,
    thread: ThreadId,
    wake: Waker,
    changed: Condvar,
    pending: AtomicBool,
    notifications: AtomicUsize,
    state: Mutex<IngressState>,
    #[cfg(test)]
    locks: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    wakes: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    handle_clones: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    handle_drops: std::sync::atomic::AtomicUsize,
}

impl Ingress {
    fn lock(&self) -> MutexGuard<'_, IngressState> {
        #[cfg(test)]
        self.locks.fetch_add(1, Ordering::Relaxed);
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn notify(&self) {
        #[cfg(test)]
        self.wakes.fetch_add(1, Ordering::Relaxed);
        self.wake.wake_by_ref();
    }

    fn notify_if_open(&self) -> bool {
        if self
            .notifications
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                state.checked_add(1).filter(|next| *next < CLOSING)
            })
            .is_err()
        {
            return false;
        }
        let _notification = Notification(self);
        self.notify();
        true
    }
}

struct Notification<'owner>(&'owner Ingress);

impl Drop for Notification<'_> {
    fn drop(&mut self) {
        if self.0.notifications.fetch_sub(1, Ordering::AcqRel) & CLOSING != 0 {
            let _state = self.0.lock();
            self.0.changed.notify_one();
        }
    }
}

pub struct DispatchHandle(Arc<Ingress>);

impl Clone for DispatchHandle {
    #[inline]
    fn clone(&self) -> Self {
        #[cfg(test)]
        self.0.handle_clones.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(&self.0))
    }
}

#[cfg(test)]
impl Drop for DispatchHandle {
    fn drop(&mut self) {
        self.0.handle_drops.fetch_add(1, Ordering::Relaxed);
    }
}

impl DispatchHandle {
    pub(crate) fn owner(&self) -> DispatchOwner {
        self.0.owner
    }
    pub(crate) fn notify(&self) {
        let _ = self.0.notify_if_open();
    }

    pub fn wake(&self) -> Result<(), JsError> {
        if self.0.notify_if_open() {
            Ok(())
        } else {
            Err(JsError::error(NO_LOOP))
        }
    }

    fn schedule(&self, runnable: ReadyJob) {
        if thread::current().id() == self.0.thread {
            JOBS.with(|owner| {
                let jobs = owner
                    .get()
                    .expect("an admitted local task has a loop owner");
                let matches = jobs
                    .handle
                    .borrow()
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(&current.0, &self.0));
                if matches {
                    jobs.ready.borrow_mut().push_back(runnable);
                } else {
                    drop(runnable);
                }
            });
            return;
        }
        let mut state = self.0.lock();
        if state.closed {
            drop(state);
            drop(runnable);
            return;
        }
        state.ready.push_back(runnable);
        self.0.pending.store(true, Ordering::Release);
        let closing = state.closing;
        drop(state);
        if closing || !self.0.notify_if_open() {
            self.0.changed.notify_one();
        }
    }
}

struct Jobs {
    accepting: Cell<bool>,
    polling: Cell<bool>,
    exit: Cell<Option<u8>>,
    live: Cell<usize>,
    maximum: Cell<usize>,
    capacity: Cell<usize>,
    handle: RefCell<Option<DispatchHandle>>,
    ready: RefCell<VecDeque<ReadyJob>>,
    tasks: RefCell<Slab<Task<(), usize>>>,
}

impl Jobs {
    fn new() -> Self {
        Self {
            accepting: Cell::new(false),
            polling: Cell::new(false),
            exit: Cell::new(None),
            live: Cell::new(0),
            maximum: Cell::new(MAX_PENDING_JOBS),
            capacity: Cell::new(0),
            handle: RefCell::new(None),
            ready: RefCell::new(VecDeque::new()),
            tasks: RefCell::new(Slab::new()),
        }
    }

    fn grow(&self, required: usize, handle: &DispatchHandle) {
        if required <= self.capacity.get() {
            return;
        }
        let capacity = required.next_power_of_two().min(self.maximum.get());
        {
            let mut tasks = self.tasks.borrow_mut();
            let additional = capacity.saturating_sub(tasks.len());
            tasks.reserve(additional);
        }
        {
            let mut ready = self.ready.borrow_mut();
            let additional = capacity.saturating_sub(ready.len());
            ready.reserve(additional);
        }
        let mut state = handle.0.lock();
        let additional = capacity.saturating_sub(state.ready.len());
        state.ready.reserve(additional);
        self.capacity.set(capacity);
    }

    fn drain_ingress(&self) {
        let handle = self.handle.borrow();
        let Some(handle) = handle.as_ref() else {
            return;
        };
        if !handle.0.pending.load(Ordering::Acquire)
            || !handle.0.pending.swap(false, Ordering::AcqRel)
        {
            return;
        }
        let mut state = handle.0.lock();
        self.ready.borrow_mut().append(&mut state.ready);
    }

    fn run(&self, runnable: ReadyJob) {
        let _running = RunningJob {
            owner: self,
            key: *runnable.metadata(),
        };
        runnable.run();
    }

    fn shutdown(&self) {
        self.accepting.set(false);
        let Some(handle) = self.handle.borrow().as_ref().cloned() else {
            return;
        };
        self.stop_admission();
        let mut tasks = std::mem::take(&mut *self.tasks.borrow_mut());
        for task in tasks.drain() {
            drop(task);
        }
        while self.live.get() != 0 {
            self.drain_ingress();
            let runnable = self.ready.borrow_mut().pop_front();
            if let Some(runnable) = runnable {
                runnable.run();
                continue;
            }
            let mut state = handle.0.lock();
            while state.ready.is_empty() && self.live.get() != 0 {
                state = handle
                    .0
                    .changed
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let late = {
            let mut state = handle.0.lock();
            while handle.0.notifications.load(Ordering::Acquire) != CLOSING {
                state = handle
                    .0
                    .changed
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            state.closed = true;
            std::mem::take(&mut state.ready)
        };
        let mut local = std::mem::take(&mut *self.ready.borrow_mut());
        drop(late);
        local.clear();
        *self.ready.borrow_mut() = local;
        *self.tasks.borrow_mut() = tasks;
        self.handle.borrow_mut().take();
        self.polling.set(false);
        self.capacity.set(0);
    }

    fn stop_admission(&self) {
        self.accepting.set(false);
        if let Some(handle) = self.handle.borrow().as_ref() {
            handle.0.notifications.fetch_or(CLOSING, Ordering::AcqRel);
            handle.0.lock().closing = true;
        }
    }
}

struct RunningJob<'owner> {
    owner: &'owner Jobs,
    key: usize,
}

impl Drop for RunningJob<'_> {
    fn drop(&mut self) {
        let finished = self
            .owner
            .tasks
            .borrow()
            .get(self.key)
            .is_some_and(Task::is_finished);
        if finished {
            let task = self.owner.tasks.borrow_mut().remove(self.key);
            drop(task);
        }
    }
}

thread_local! {
    static JOBS: OnceCell<Jobs> = const { OnceCell::new() };
}

pub(crate) struct Owner;

impl Owner {
    pub(crate) fn enter(wake: Waker) -> Result<Self, JsError> {
        JOBS.with(|owner| {
            let jobs = owner.get_or_init(Jobs::new);
            if jobs.handle.borrow().is_some() {
                return Err(JsError::error(
                    "A JavaScript event loop is already running on this thread.",
                ));
            }
            let identity = DispatchOwner::new().ok_or_else(|| {
                crate::range_error("native event-loop owner identity range is exhausted")
            })?;
            *jobs.handle.borrow_mut() = Some(DispatchHandle(Arc::new(Ingress {
                owner: identity,
                thread: thread::current().id(),
                wake,
                changed: Condvar::new(),
                pending: AtomicBool::new(false),
                notifications: AtomicUsize::new(0),
                state: Mutex::new(IngressState {
                    ready: VecDeque::new(),
                    closing: false,
                    closed: false,
                }),
                #[cfg(test)]
                locks: std::sync::atomic::AtomicUsize::new(0),
                #[cfg(test)]
                wakes: std::sync::atomic::AtomicUsize::new(0),
                #[cfg(test)]
                handle_clones: std::sync::atomic::AtomicUsize::new(0),
                #[cfg(test)]
                handle_drops: std::sync::atomic::AtomicUsize::new(0),
            })));
            jobs.exit.set(None);
            jobs.accepting.set(true);
            Ok(Self)
        })
    }

    pub(crate) fn stop_admission(&self) {
        JOBS.with(|owner| {
            owner
                .get()
                .expect("an active loop owns admission")
                .stop_admission()
        });
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        JOBS.with(|owner| owner.get().expect("a live loop owns its jobs").shutdown());
    }
}

struct JobLifetime;

impl Drop for JobLifetime {
    fn drop(&mut self) {
        JOBS.with(|owner| {
            let jobs = owner
                .get()
                .expect("a local task releases on its owning thread");
            jobs.live.set(jobs.live.get() - 1);
        });
    }
}

pub(crate) fn active() -> bool {
    JOBS.with(|owner| {
        owner
            .get()
            .is_some_and(|jobs| jobs.handle.borrow().is_some())
    })
}

pub fn ensure_running() -> Result<(), JsError> {
    JOBS.with(|owner| {
        if owner.get().is_some_and(|jobs| jobs.accepting.get()) {
            Ok(())
        } else {
            Err(JsError::error(NO_LOOP))
        }
    })
}

pub fn dispatch_handle() -> Result<DispatchHandle, JsError> {
    JOBS.with(|owner| {
        owner
            .get()
            .filter(|jobs| jobs.accepting.get())
            .and_then(|jobs| jobs.handle.borrow().clone())
            .ok_or_else(|| JsError::error(NO_LOOP))
    })
}

pub fn dispatch_owner() -> Result<DispatchOwner, JsError> {
    JOBS.with(|owner| {
        owner
            .get()
            .filter(|jobs| jobs.accepting.get())
            .and_then(|jobs| jobs.handle.borrow().as_ref().map(DispatchHandle::owner))
            .ok_or_else(|| JsError::error(NO_LOOP))
    })
}

pub(crate) fn can_admit() -> Result<(), JsError> {
    JOBS.with(|owner| {
        let jobs = owner
            .get()
            .filter(|jobs| jobs.accepting.get())
            .ok_or_else(|| JsError::error(NO_LOOP))?;
        if jobs.tasks.borrow().len() >= jobs.maximum.get() {
            return Err(crate::range_error(
                "pending Promise jobs exceed the finite queue limit",
            ));
        }
        Ok(())
    })
}

pub(crate) fn request_exit(code: u8) -> bool {
    JOBS.with(|owner| {
        let Some(jobs) = owner.get().filter(|jobs| jobs.handle.borrow().is_some()) else {
            return false;
        };
        if jobs.exit.get().is_none() {
            jobs.exit.set(Some(code));
        }
        true
    })
}

pub(crate) fn exit_requested() -> bool {
    JOBS.with(|owner| owner.get().is_some_and(|jobs| jobs.exit.get().is_some()))
}

pub(crate) fn take_exit() -> Option<u8> {
    JOBS.with(|owner| owner.get().and_then(|jobs| jobs.exit.take()))
}

pub(crate) fn enqueue(job: impl Future<Output = ()> + 'static) -> Result<(), JsError> {
    admit(job, false)
}

pub(crate) fn start(job: impl Future<Output = ()> + 'static) -> Result<(), JsError> {
    admit(job, true)
}

fn admit(job: impl Future<Output = ()> + 'static, eager: bool) -> Result<(), JsError> {
    JOBS.with(|owner| {
        let jobs = owner
            .get()
            .filter(|jobs| jobs.accepting.get())
            .ok_or_else(|| JsError::error(NO_LOOP))?;
        let (key, count) = {
            let tasks = jobs.tasks.borrow();
            (tasks.vacant_key(), tasks.len())
        };
        if count >= jobs.maximum.get() {
            return Err(crate::range_error(
                "pending Promise jobs exceed the finite queue limit",
            ));
        }
        let handle = jobs
            .handle
            .borrow()
            .as_ref()
            .expect("active job owner")
            .clone();
        jobs.grow(count + 1, &handle);
        jobs.live.set(jobs.live.get() + 1);
        let lifetime = JobLifetime;
        let (runnable, task) = async_task::Builder::new().metadata(key).spawn_local(
            move |_| job.map(move |()| drop(lifetime)),
            move |runnable| handle.schedule(runnable),
        );
        assert_eq!(jobs.tasks.borrow_mut().insert(task), key);
        if eager {
            jobs.run(runnable);
        } else {
            jobs.ready.borrow_mut().push_back(runnable);
        }
        Ok(())
    })
}

pub(crate) fn has_ready() -> bool {
    JOBS.with(|owner| {
        let Some(jobs) = owner.get() else {
            return false;
        };
        if !jobs.ready.borrow().is_empty() {
            return true;
        }
        jobs.handle
            .borrow()
            .as_ref()
            .is_some_and(|handle| handle.0.pending.load(Ordering::Acquire))
    })
}

struct Polling<'owner>(&'owner Cell<bool>);

impl Drop for Polling<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

pub(crate) fn poll() -> bool {
    JOBS.with(|owner| {
        let Some(jobs) = owner.get() else {
            return false;
        };
        if jobs.polling.replace(true) {
            panic!("Promise job polling cannot reenter");
        }
        let _polling = Polling(&jobs.polling);
        jobs.drain_ingress();
        let count = jobs.ready.borrow().len().min(MAX_JOBS_PER_TURN);
        for _ in 0..count {
            if jobs.exit.get().is_some() {
                break;
            }
            let runnable = jobs
                .ready
                .borrow_mut()
                .pop_front()
                .expect("captured ready frontier");
            jobs.run(runnable);
        }
        count != 0
    })
}

pub(crate) fn checkpoint() -> bool {
    let idle = JOBS.with(|owner| owner.get().is_some_and(|jobs| !jobs.polling.get()));
    if !idle {
        return false;
    }
    let mut did_work = false;
    while has_ready() && !exit_requested() {
        did_work |= poll();
    }
    did_work
}

#[cfg(test)]
mod tests;
