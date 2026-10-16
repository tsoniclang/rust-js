use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread, ThreadId};

use tsonic_rust_runtime::dispatch::DispatchOwner;
use tsonic_rust_runtime::dispatch::{DispatchContexts, DispatchEnd, DispatchPhase};
use tsonic_rust_runtime::TsonicResult;

pub use crate::promise_jobs::{
    dispatch_handle, dispatch_owner, ensure_running, require_dispatch_owner, DispatchHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopLifecycle {
    Enter(DispatchOwner),
    Shutdown(DispatchOwner),
}

pub trait EventLoopDriver {
    type Error;

    fn admission_error(error: tsonic_rust_runtime::JsError) -> Self::Error;
    fn lifecycle(&mut self, transition: LoopLifecycle) -> Result<(), Self::Error>;
    fn poll(&mut self) -> Result<bool, Self::Error>;
    fn has_work(&self) -> bool;
    fn wait(&mut self) -> Result<(), Self::Error>;
    fn waker(&mut self) -> Result<Waker, Self::Error>;
}

struct LoopWake {
    ready: AtomicBool,
    driver: DispatchHandle,
    thread: ThreadId,
}

impl Wake for LoopWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.ready.store(true, Ordering::Release);
        if thread::current().id() != self.thread {
            self.driver.notify();
        }
    }
}

struct ThreadWake(Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

struct TimerDriver<TContexts> {
    contexts: TContexts,
}

impl<TContexts: DispatchContexts> EventLoopDriver for TimerDriver<TContexts>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    type Error = TContexts::Error;

    fn admission_error(error: tsonic_rust_runtime::JsError) -> Self::Error {
        tsonic_rust_runtime::TsonicError::from(error).into()
    }

    fn lifecycle(&mut self, transition: LoopLifecycle) -> Result<(), Self::Error> {
        match transition {
            LoopLifecycle::Enter(owner) => {
                if !self.contexts.enter(owner)
                    || !crate::timers::with_default(|timers| timers.enter(owner))
                {
                    return Err(Self::admission_error(tsonic_rust_runtime::JsError::error("native dispatch context belongs to another event loop or has exhausted its generations")));
                }
            }
            LoopLifecycle::Shutdown(owner) => {
                self.contexts.close(owner);
                crate::timers::with_default(|timers| timers.close(owner));
            }
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<bool, Self::Error> {
        crate::timers::with_default(|native| {
            poll_phase_with_jobs(
                &tsonic_rust_runtime::dispatch::prepend(native, &self.contexts),
                crate::timers::PHASE,
            )
        })
    }

    fn has_work(&self) -> bool {
        crate::timers::has_timers() || self.contexts.has_work()
    }

    fn wait(&mut self) -> Result<(), Self::Error> {
        let native = crate::timers::next_timer_delay();
        let selected = self.contexts.next_delay();
        let delay = match (native, selected) {
            (Some(native), Some(selected)) => Some(native.min(selected)),
            (Some(delay), None) | (None, Some(delay)) => Some(delay),
            (None, None) => None,
        };
        match delay {
            Some(delay) => thread::park_timeout(delay),
            None => thread::park(),
        }
        Ok(())
    }

    fn waker(&mut self) -> Result<Waker, Self::Error> {
        Ok(Waker::from(Arc::new(ThreadWake(thread::current()))))
    }
}

pub const STOPPED_BEFORE_COMPLETION: &str =
    "The event loop stopped on a requested exit before the awaited future completed.";
pub const STOPPED_ON_REQUESTED_EXIT: &str = "The event loop stopped on a requested exit.";

pub fn poll_phase_with_jobs<TContexts: DispatchContexts>(
    contexts: &TContexts,
    phase: DispatchPhase,
) -> Result<bool, TContexts::Error> {
    let mut did_work = crate::promise_jobs::checkpoint();
    if crate::promise_jobs::exit_requested() {
        return Ok(did_work);
    }
    let frontier = contexts.prepare(phase)?;
    while contexts.next_ready(&frontier).is_some() {
        did_work |= contexts.poll_next(&frontier)?;
        did_work |= crate::promise_jobs::checkpoint();
        if crate::promise_jobs::exit_requested() {
            break;
        }
    }
    Ok(did_work)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopExitStatus {
    requested: Option<u8>,
}

impl LoopExitStatus {
    pub fn exit_requested(&self) -> bool {
        self.requested.is_some()
    }
}

impl From<LoopExitStatus> for std::process::ExitCode {
    fn from(status: LoopExitStatus) -> Self {
        match status.requested {
            Some(code) => Self::from(code),
            None => Self::SUCCESS,
        }
    }
}

pub fn block_on<Output>(future: impl Future<Output = Output>) -> TsonicResult<Output> {
    let (output, status) = block_on_with_contexts(
        future,
        DispatchEnd::<tsonic_rust_runtime::TsonicError>::new(),
    )?;
    match output {
        Some(_) if status.exit_requested() => {
            Err(tsonic_rust_runtime::JsError::error(STOPPED_ON_REQUESTED_EXIT).into())
        }
        Some(output) => Ok(output),
        None => Err(tsonic_rust_runtime::JsError::error(STOPPED_BEFORE_COMPLETION).into()),
    }
}

pub fn run_event_loop() -> TsonicResult<()> {
    let status = run_with_contexts(DispatchEnd::<tsonic_rust_runtime::TsonicError>::new())?;
    if status.exit_requested() {
        return Err(tsonic_rust_runtime::JsError::error(STOPPED_ON_REQUESTED_EXIT).into());
    }
    Ok(())
}

pub fn block_on_with_contexts<TOutput, TContexts: DispatchContexts>(
    future: impl Future<Output = TOutput>,
    contexts: TContexts,
) -> Result<(Option<TOutput>, LoopExitStatus), TContexts::Error>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    block_on_with_driver(future, &mut TimerDriver { contexts })
}

pub fn run_with_contexts<TContexts: DispatchContexts>(
    contexts: TContexts,
) -> Result<LoopExitStatus, TContexts::Error>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    run_with_driver(&mut TimerDriver { contexts })
}

pub fn run_entry_with_contexts<TOutput, TContexts: DispatchContexts>(
    entry: impl FnOnce() -> TOutput,
    contexts: TContexts,
) -> Result<(TOutput, LoopExitStatus), TContexts::Error>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    run_entry_with_driver(entry, &mut TimerDriver { contexts })
}

pub fn block_on_with_driver<Output, Driver: EventLoopDriver>(
    future: impl Future<Output = Output>,
    driver: &mut Driver,
) -> Result<(Option<Output>, LoopExitStatus), Driver::Error> {
    drive(Some(future), None, driver)
}

pub fn run_with_driver<Driver: EventLoopDriver>(
    driver: &mut Driver,
) -> Result<LoopExitStatus, Driver::Error> {
    drive::<std::future::Pending<()>, Driver>(None, None, driver).map(|(_, status)| status)
}

pub fn run_entry_with_driver<TOutput, Driver: EventLoopDriver>(
    entry: impl FnOnce() -> TOutput,
    driver: &mut Driver,
) -> Result<(TOutput, LoopExitStatus), Driver::Error> {
    let mut entry = Some(entry);
    let mut output = None;
    let mut run = || output = Some((entry.take().expect("the entry runs once"))());
    let (_, status) = drive::<std::future::Pending<()>, Driver>(None, Some(&mut run), driver)?;
    Ok((
        output.expect("the entry runs inside the owner before the loop drains"),
        status,
    ))
}

fn drive<Root: Future, Driver: EventLoopDriver>(
    root: Option<Root>,
    entry: Option<&mut dyn FnMut()>,
    driver: &mut Driver,
) -> Result<(Option<Root::Output>, LoopExitStatus), Driver::Error> {
    if crate::promise_jobs::active() {
        return Err(Driver::admission_error(
            tsonic_rust_runtime::JsError::error(
                "A JavaScript event loop is already running on this thread.",
            ),
        ));
    }
    if root.is_none() && entry.is_none() && !driver.has_work() {
        return Ok((None, LoopExitStatus::default()));
    }
    let driver_waker = driver.waker()?;
    let owner = crate::promise_jobs::Owner::enter(driver_waker).map_err(Driver::admission_error)?;
    let identity = dispatch_owner().map_err(Driver::admission_error)?;
    let mut scope = LoopScope {
        driver,
        owner,
        identity,
        closed: false,
    };
    let output = {
        let mut root = pin!(root);
        let _admission = CloseRootAdmission(&scope.owner);
        run_turns(scope.driver, identity, root.as_mut().as_pin_mut(), entry)
    };
    let output = match output {
        Ok(output) => scope.close().map(|()| output),
        Err(error) => Err(error),
    };
    drop(scope);
    let status = LoopExitStatus {
        requested: crate::promise_jobs::take_exit(),
    };
    output.map(|output| (output, status))
}

fn run_turns<Root: Future, Driver: EventLoopDriver>(
    driver: &mut Driver,
    identity: DispatchOwner,
    mut root: Option<Pin<&mut Root>>,
    entry: Option<&mut dyn FnMut()>,
) -> Result<Option<Root::Output>, Driver::Error> {
    driver.lifecycle(LoopLifecycle::Enter(identity))?;
    if let Some(entry) = entry {
        entry();
        crate::promise_jobs::checkpoint();
    }
    let wake = if root.is_some() {
        Some(Arc::new(LoopWake {
            ready: AtomicBool::new(true),
            driver: dispatch_handle().map_err(Driver::admission_error)?,
            thread: thread::current().id(),
        }))
    } else {
        None
    };
    let waker = wake.as_ref().map(|wake| Waker::from(Arc::clone(wake)));
    let mut context = Context::from_waker(waker.as_ref().unwrap_or(Waker::noop()));
    let mut output = None;
    loop {
        if crate::promise_jobs::exit_requested() {
            return Ok(output);
        }
        let driver_work = driver.poll()?;
        if crate::promise_jobs::exit_requested() {
            return Ok(output);
        }
        let root_polled = wake
            .as_ref()
            .is_some_and(|wake| wake.ready.swap(false, Ordering::AcqRel))
            && root.is_some();
        if root_polled {
            if let Some(future) = &mut root {
                if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                    output = Some(value);
                    root = None;
                }
            }
        }
        let job_work = crate::promise_jobs::poll();
        if crate::promise_jobs::exit_requested() {
            return Ok(output);
        }
        let ready = crate::promise_jobs::has_ready();
        if root.is_none() && !driver.has_work() && !ready {
            return Ok(output);
        }
        if !driver_work
            && !root_polled
            && !job_work
            && !ready
            && !wake
                .as_ref()
                .is_some_and(|wake| wake.ready.load(Ordering::Acquire))
        {
            driver.wait()?;
        }
    }
}

struct CloseRootAdmission<'owner>(&'owner crate::promise_jobs::Owner);

impl Drop for CloseRootAdmission<'_> {
    fn drop(&mut self) {
        self.0.stop_admission();
    }
}

struct LoopScope<'driver, Driver: EventLoopDriver> {
    driver: &'driver mut Driver,
    owner: crate::promise_jobs::Owner,
    identity: DispatchOwner,
    closed: bool,
}

impl<Driver: EventLoopDriver> LoopScope<'_, Driver> {
    fn close(&mut self) -> Result<(), Driver::Error> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.driver
            .lifecycle(LoopLifecycle::Shutdown(self.identity))
    }
}

impl<Driver: EventLoopDriver> Drop for LoopScope<'_, Driver> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests;
