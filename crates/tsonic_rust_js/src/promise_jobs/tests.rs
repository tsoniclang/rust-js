use super::*;
use std::cell::{Cell, RefCell};
use std::future::{pending, poll_fn};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::mpsc;
use std::task::Poll;

fn owner() -> Owner {
    Owner::enter(Waker::noop().clone()).unwrap()
}

struct DropProbe {
    drops: Rc<Cell<usize>>,
    thread: ThreadId,
}

impl DropProbe {
    fn new(drops: &Rc<Cell<usize>>) -> Self {
        Self {
            drops: Rc::clone(drops),
            thread: thread::current().id(),
        }
    }
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        assert_eq!(thread::current().id(), self.thread);
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn missing_owner_rejects_without_polling_retaining_or_initializing_jobs() {
    JOBS.with(|owner| assert!(owner.get().is_none()));
    let ran = Rc::new(Cell::new(false));
    let captured = Rc::clone(&ran);
    let failure = enqueue(async move { captured.set(true) }).unwrap_err();
    assert_eq!(failure.message(), NO_LOOP);
    assert!(!ran.get());
    assert_eq!(Rc::strong_count(&ran), 1);
    JOBS.with(|owner| assert!(owner.get().is_none()));
}

#[test]
fn cold_queries_and_polling_do_not_initialize_jobs() {
    JOBS.with(|owner| assert!(owner.get().is_none()));
    assert!(!has_ready());
    assert!(!poll());
    assert!(!active());
    JOBS.with(|owner| assert!(owner.get().is_none()));
}

#[test]
fn no_loop_reaction_returns_the_exact_error_without_running_the_callback() {
    let calls = Rc::new(Cell::new(0));
    let captured = Rc::clone(&calls);
    let pending = crate::promise::JsPromise::<i32>::resolved(1).then(
        tsonic_rust_runtime::Callable::new(move |(value,)| {
            captured.set(1);
            Ok(value)
        }),
        None,
    );
    let failure = tsonic_rust_runtime::async_runtime::block_on(pending.into_result()).unwrap_err();
    assert_eq!(
        tsonic_rust_runtime::ErrorObject::error_message(&failure.error_value()),
        NO_LOOP
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(Rc::strong_count(&calls), 1);
    assert!(!active());
    JOBS.with(|owner| assert!(owner.get().is_none()));
}

#[test]
fn admission_reuses_one_owner_and_queries_preserve_its_identity() {
    let _owner = owner();
    enqueue(async {}).unwrap();
    let first = JOBS.with(|owner| std::ptr::from_ref(owner.get().unwrap()));
    assert!(has_ready());
    enqueue(async {}).unwrap();
    assert!(poll());
    assert!(!has_ready());
    JOBS.with(|owner| {
        let current = owner.get().unwrap();
        assert_eq!(first, std::ptr::from_ref(current));
        assert_eq!(current.live.get(), 0);
        assert!(current.tasks.borrow().is_empty());
    });
}

#[test]
fn a_full_owner_rejects_before_polling_and_releases_the_rejected_capture() {
    let _owner = owner();
    JOBS.with(|owner| owner.get().unwrap().maximum.set(2));
    enqueue(pending()).unwrap();
    enqueue(pending()).unwrap();
    let ran = Rc::new(Cell::new(false));
    let captured = Rc::clone(&ran);
    let failure = enqueue(async move { captured.set(true) }).unwrap_err();
    assert_eq!(failure.kind(), tsonic_rust_runtime::JsErrorKind::RangeError);
    assert!(!ran.get());
    assert_eq!(Rc::strong_count(&ran), 1);
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(jobs.live.get(), 2);
        assert_eq!(jobs.tasks.borrow().len(), 2);
        assert_eq!(jobs.ready.borrow().len(), 2);
    });
}

#[test]
fn a_report_a_full_owner_cannot_admit_is_written_and_requests_exit_one() {
    let _owner = owner();
    JOBS.with(|owner| owner.get().unwrap().maximum.set(1));
    enqueue(pending()).unwrap();
    crate::promise::JsPromise::<i32>::rejected(crate::JsValue::String(String::from("full queue")))
        .observe_discarded();
    assert_eq!(crate::promise::take_reports(), "full queue\n");
    assert!(exit_requested());
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(jobs.exit.get(), Some(1));
        assert_eq!(jobs.tasks.borrow().len(), 1);
    });
}

#[test]
fn rejected_admission_settles_the_returned_promise_and_releases_captures() {
    use crate::promise::JsPromise;
    use tsonic_rust_runtime::Callable;

    let _owner = owner();
    JOBS.with(|owner| owner.get().unwrap().maximum.set(1));
    enqueue(pending()).unwrap();
    let ran = Rc::new(Cell::new(false));
    let captured = Rc::clone(&ran);
    let result = JsPromise::<i32>::resolved(1).then(
        Callable::new(move |(value,)| {
            captured.set(true);
            Ok(value)
        }),
        None,
    );
    let failure = tsonic_rust_runtime::async_runtime::block_on(result.into_result()).unwrap_err();
    assert!(failure.is_error_kind(tsonic_rust_runtime::JsErrorKind::RangeError));
    assert!(!ran.get());
    assert_eq!(Rc::strong_count(&ran), 1);
}

#[test]
fn local_fifo_defers_reentrant_jobs_behind_already_admitted_work() {
    let _owner = owner();
    let order = Rc::new(RefCell::new(Vec::new()));
    let outer = Rc::clone(&order);
    enqueue(async move {
        outer.borrow_mut().push(1);
        let inner = Rc::clone(&outer);
        enqueue(async move { inner.borrow_mut().push(4) }).unwrap();
        outer.borrow_mut().push(2);
    })
    .unwrap();
    let second = Rc::clone(&order);
    enqueue(async move { second.borrow_mut().push(3) }).unwrap();
    assert!(poll());
    assert_eq!(*order.borrow(), [1, 2, 3]);
    assert!(has_ready());
    assert_eq!(Rc::strong_count(&order), 2);
    assert!(poll());
    assert_eq!(*order.borrow(), [1, 2, 3, 4]);
    assert!(!has_ready());
    assert_eq!(Rc::strong_count(&order), 1);
}

#[test]
fn an_unwound_poll_releases_its_borrow_and_preserves_uninvoked_work() {
    let _owner = owner();
    let ran = Rc::new(Cell::new(false));
    let captured = Rc::clone(&ran);
    enqueue(async { panic!("original Promise job failure") }).unwrap();
    enqueue(async move { captured.set(true) }).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(poll)).is_err());
    assert!(!ran.get());
    assert!(has_ready());
    assert_eq!(Rc::strong_count(&ran), 2);
    assert!(poll());
    assert!(ran.get());
    assert!(!has_ready());
    assert_eq!(Rc::strong_count(&ran), 1);
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(jobs.live.get(), 0);
        assert!(jobs.tasks.borrow().is_empty());
    });
}

#[test]
fn recursive_polling_rejects_without_losing_the_outer_owner() {
    let _owner = owner();
    let rejected = Rc::new(Cell::new(false));
    let captured = Rc::clone(&rejected);
    enqueue(async move { captured.set(catch_unwind(AssertUnwindSafe(poll)).is_err()) }).unwrap();
    assert!(poll());
    assert!(rejected.get());
    assert!(!has_ready());
    assert_eq!(Rc::strong_count(&rejected), 1);
}

#[test]
fn warmed_local_rescheduling_takes_no_ingress_lock_or_wake_or_queue_growth() {
    let _owner = owner();
    let polls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&polls);
    enqueue(poll_fn(move |context| {
        counter.set(counter.get() + 1);
        if counter.get() == 1 {
            for _ in 0..100 {
                context.waker().wake_by_ref();
            }
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }))
    .unwrap();
    let handle = dispatch_handle().unwrap();
    let locks = handle.0.locks.load(Ordering::Relaxed);
    let wakes = handle.0.wakes.load(Ordering::Relaxed);
    let clones = handle.0.handle_clones.load(Ordering::Relaxed);
    let drops = handle.0.handle_drops.load(Ordering::Relaxed);
    let capacities = JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        (
            jobs.ready.borrow().capacity(),
            jobs.tasks.borrow().capacity(),
        )
    });
    assert!(poll());
    assert_eq!(polls.get(), 1);
    assert_eq!(handle.0.handle_clones.load(Ordering::Relaxed), clones);
    assert_eq!(handle.0.handle_drops.load(Ordering::Relaxed), drops);
    JOBS.with(|owner| assert_eq!(owner.get().unwrap().ready.borrow().len(), 1));
    assert!(poll());
    assert_eq!(polls.get(), 2);
    assert!(!has_ready());
    assert_eq!(handle.0.locks.load(Ordering::Relaxed), locks);
    assert_eq!(handle.0.wakes.load(Ordering::Relaxed), wakes);
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(
            (
                jobs.ready.borrow().capacity(),
                jobs.tasks.borrow().capacity()
            ),
            capacities
        );
    });
}

#[test]
fn foreign_wakes_coalesce_and_do_not_poll_unrelated_pending_tasks() {
    let _owner = owner();
    let first_polls = Rc::new(Cell::new(0));
    let second_polls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&first_polls);
    let (sender, receiver) = mpsc::sync_channel(1);
    enqueue(poll_fn(move |context| {
        counter.set(counter.get() + 1);
        if counter.get() == 1 {
            sender.send(context.waker().clone()).unwrap();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }))
    .unwrap();
    let counter = Rc::clone(&second_polls);
    enqueue(poll_fn(move |_| {
        counter.set(counter.get() + 1);
        Poll::Pending
    }))
    .unwrap();
    assert!(poll());
    let handle = dispatch_handle().unwrap();
    let wakes = handle.0.wakes.load(Ordering::Relaxed);
    thread::spawn(move || {
        let waker = receiver.recv().unwrap();
        for _ in 0..100 {
            waker.wake_by_ref();
        }
    })
    .join()
    .unwrap();
    assert_eq!(handle.0.wakes.load(Ordering::Relaxed), wakes + 1);
    assert!(has_ready());
    assert!(poll());
    assert_eq!(first_polls.get(), 2);
    assert_eq!(second_polls.get(), 1);
    assert!(!has_ready());
}

#[test]
fn owner_shutdown_drops_ready_and_pending_frames_on_the_owner_thread() {
    let drops = Rc::new(Cell::new(0));
    {
        let _owner = owner();
        let first = DropProbe::new(&drops);
        enqueue(async move {
            pending::<()>().await;
            drop(first);
        })
        .unwrap();
        assert!(poll());
        let second = DropProbe::new(&drops);
        enqueue(async move {
            pending::<()>().await;
            drop(second);
        })
        .unwrap();
        assert_eq!(drops.get(), 0);
    }
    assert_eq!(drops.get(), 2);
    assert_eq!(Rc::strong_count(&drops), 1);
    assert!(!active());
    assert!(!has_ready());
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(jobs.live.get(), 0);
        assert!(jobs.tasks.borrow().is_empty());
    });
}

#[test]
fn capture_destruction_cannot_reenter_admission_during_shutdown() {
    struct ReentrantDrop(Rc<Cell<bool>>);
    impl Drop for ReentrantDrop {
        fn drop(&mut self) {
            self.0
                .set(enqueue(async {}).unwrap_err().message() == NO_LOOP);
        }
    }
    let rejected = Rc::new(Cell::new(false));
    {
        let _owner = owner();
        let capture = ReentrantDrop(Rc::clone(&rejected));
        enqueue(async move {
            pending::<()>().await;
            drop(capture);
        })
        .unwrap();
    }
    assert!(rejected.get());
    assert_eq!(Rc::strong_count(&rejected), 1);
}

#[test]
fn late_foreign_handles_and_wakes_never_enter_the_next_generation() {
    let drops = Rc::new(Cell::new(0));
    let (handle, waker) = {
        let _owner = owner();
        let (sender, receiver) = mpsc::sync_channel(1);
        let probe = DropProbe::new(&drops);
        enqueue(async move {
            poll_fn(|context| {
                sender.send(context.waker().clone()).unwrap();
                Poll::<()>::Pending
            })
            .await;
            drop(probe);
        })
        .unwrap();
        assert!(poll());
        (dispatch_handle().unwrap(), receiver.recv().unwrap())
    };
    assert_eq!(drops.get(), 1);
    let _owner = owner();
    let current = dispatch_handle().unwrap();
    assert!(!Arc::ptr_eq(&handle.0, &current.0));
    thread::spawn(move || {
        assert_eq!(handle.wake().unwrap_err().message(), NO_LOOP);
        waker.wake();
    })
    .join()
    .unwrap();
    assert!(!has_ready());
    assert_eq!(current.0.wakes.load(Ordering::Relaxed), 0);
}

#[test]
fn released_slots_and_queue_storage_are_reused_with_finite_growth() {
    let _owner = owner();
    JOBS.with(|owner| owner.get().unwrap().maximum.set(4));
    for _ in 0..4 {
        enqueue(async {}).unwrap();
    }
    let capacities = JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        (
            jobs.tasks.borrow().capacity(),
            jobs.ready.borrow().capacity(),
            jobs.capacity.get(),
        )
    });
    assert!(poll());
    for _ in 0..20 {
        for _ in 0..4 {
            enqueue(async {}).unwrap();
        }
        assert!(poll());
    }
    JOBS.with(|owner| {
        let jobs = owner.get().unwrap();
        assert_eq!(
            (
                jobs.tasks.borrow().capacity(),
                jobs.ready.borrow().capacity(),
                jobs.capacity.get()
            ),
            capacities
        );
        assert_eq!(jobs.live.get(), 0);
        assert!(jobs.tasks.borrow().is_empty());
    });
}

#[test]
fn racing_foreign_wakes_and_owner_cancellation_release_non_send_frames_exactly_once() {
    for _ in 0..64 {
        let drops = Rc::new(Cell::new(0));
        let activation = owner();
        let (sender, receiver) = mpsc::sync_channel(1);
        let capture = DropProbe::new(&drops);
        enqueue(async move {
            poll_fn(|context| {
                sender.send(context.waker().clone()).unwrap();
                Poll::<()>::Pending
            })
            .await;
            drop(capture);
        })
        .unwrap();
        assert!(poll());
        let waker = receiver.recv().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let foreign = Arc::clone(&barrier);
        let worker = thread::spawn(move || {
            foreign.wait();
            for _ in 0..64 {
                waker.wake_by_ref();
            }
        });
        barrier.wait();
        drop(activation);
        worker.join().unwrap();
        assert_eq!(drops.get(), 1);
        assert_eq!(Rc::strong_count(&drops), 1);
        assert!(!active());
        assert!(!has_ready());
    }
}

#[test]
fn shutdown_waits_for_an_admitted_native_notification_before_releasing_the_generation() {
    use std::task::Wake;
    struct BlockingWake {
        started: mpsc::SyncSender<()>,
        release: Mutex<mpsc::Receiver<()>>,
        finished: Arc<AtomicBool>,
    }
    impl Wake for BlockingWake {
        fn wake(self: Arc<Self>) {
            self.wake_by_ref();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.started.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
            self.finished.store(true, Ordering::Release);
        }
    }
    let (started, entered) = mpsc::sync_channel(1);
    let (release, released) = mpsc::sync_channel(1);
    let finished = Arc::new(AtomicBool::new(false));
    let owner = Owner::enter(Waker::from(Arc::new(BlockingWake {
        started,
        release: Mutex::new(released),
        finished: Arc::clone(&finished),
    })))
    .unwrap();
    let handle = dispatch_handle().unwrap();
    let ingress = Arc::clone(&handle.0);
    let notifier = thread::spawn(move || handle.wake().unwrap());
    entered.recv().unwrap();
    let releaser = thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while ingress.notifications.load(Ordering::Acquire) & CLOSING == 0
            && std::time::Instant::now() < deadline
        {
            thread::yield_now();
        }
        release.send(()).unwrap();
        assert_ne!(ingress.notifications.load(Ordering::Acquire) & CLOSING, 0);
    });
    drop(owner);
    assert!(finished.load(Ordering::Acquire));
    releaser.join().unwrap();
    notifier.join().unwrap();
    assert!(!active());
}
