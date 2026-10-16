use super::*;
use crate::promise::JsPromise;
use std::cell::{Cell, RefCell};
use std::future::{pending, poll_fn};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use tsonic_rust_runtime::{Callable, JsError};

mod root_lifetime;

#[test]
fn ready_root_retains_output_while_reentrant_reactions_drain() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&order);
    let result = block_on(async move {
        observed.borrow_mut().push(1);
        let callback = Rc::clone(&observed);
        drop(JsPromise::<i32>::resolved(7).then(
            Callable::new(move |(value,)| {
                callback.borrow_mut().push(2);
                let nested = Rc::clone(&callback);
                drop(JsPromise::<i32>::resolved(value).then(
                    Callable::new(move |(value,)| {
                        nested.borrow_mut().push(4);
                        Ok(value)
                    }),
                    None,
                ));
                Ok(value)
            }),
            None,
        ));
        observed.borrow_mut().push(3);
        29
    })
    .unwrap();
    assert_eq!(result, 29);
    assert_eq!(*order.borrow(), [1, 3, 2, 4]);
    assert_eq!(Rc::strong_count(&order), 1);
}

#[test]
fn a_pending_job_is_not_a_resource_reference_and_releases_its_capture() {
    let capture = Rc::new(Cell::new(0));
    let observed = Rc::clone(&capture);
    assert_eq!(
        block_on(async move {
            crate::promise_jobs::enqueue(async move {
                observed.set(1);
                pending::<()>().await;
                observed.set(2);
            })
            .unwrap();
            7
        })
        .unwrap(),
        7
    );
    assert_eq!(capture.get(), 1);
    assert_eq!(Rc::strong_count(&capture), 1);
    assert!(!crate::promise_jobs::active());
}

#[test]
fn retirement_drops_the_reaction_without_settling_its_source_or_derived_promise() {
    let source = JsPromise::<i32>::pending();
    let invoked = Rc::new(Cell::new(false));
    let observed = Rc::clone(&invoked);
    let reaction = block_on(async {
        source.clone().then(
            Callable::new(move |(value,)| {
                observed.set(true);
                Ok(value)
            }),
            None,
        )
    })
    .unwrap();
    let mut source_pending = std::pin::pin!(source.clone().into_result());
    let mut reaction_pending = std::pin::pin!(reaction.into_result());
    let mut context = Context::from_waker(Waker::noop());
    assert!(source_pending.as_mut().poll(&mut context).is_pending());
    assert!(reaction_pending.as_mut().poll(&mut context).is_pending());
    assert_eq!(Rc::strong_count(&invoked), 1);
    source.settle(Ok(7));
    block_on(async {}).unwrap();
    assert!(!invoked.get());
    assert!(reaction_pending.as_mut().poll(&mut context).is_pending());
    assert!(!crate::promise_jobs::active());
}

#[test]
fn drain_crosses_the_finite_per_turn_frontier_without_repolling_a_ready_root() {
    let completed = Rc::new(Cell::new(0));
    let observed = Rc::clone(&completed);
    let root_polls = Cell::new(0);
    assert_eq!(
        block_on(poll_fn(|_| {
            assert_eq!(root_polls.replace(1), 0);
            for _ in 0..1027 {
                let completed = Rc::clone(&observed);
                crate::promise_jobs::enqueue(async move { completed.set(completed.get() + 1) })
                    .unwrap();
            }
            Poll::Ready(13)
        }))
        .unwrap(),
        13
    );
    assert_eq!(completed.get(), 1027);
    assert_eq!(root_polls.get(), 1);
    assert_eq!(Rc::strong_count(&completed), 2);
}

#[test]
fn a_referenced_timer_can_finish_a_job_after_the_root_is_ready() {
    let completed = Rc::new(Cell::new(0));
    let observed = Rc::clone(&completed);
    block_on(async move {
        let promise = JsPromise::create(Callable::new(
            move |(resolve, _): (
                crate::promise::PromiseResolve<i32>,
                crate::promise::PromiseReject,
            )| {
                crate::timers::with_default(|timers| {
                    crate::timers::set_timeout_callable(
                        timers,
                        Callable::new(move |()| {
                            resolve
                                .call((crate::promise::PromiseResolution::Value(9),))
                                .expect("native Promise resolution is infallible");
                            Ok(())
                        }),
                        1.0,
                    )
                })
                .unwrap();
                Ok(())
            },
        ));
        drop(promise.then(
            Callable::new(move |(value,)| {
                observed.set(value);
                Ok(value)
            }),
            None,
        ));
    })
    .unwrap();
    assert_eq!(completed.get(), 9);
    assert_eq!(Rc::strong_count(&completed), 1);
}

#[test]
fn nested_activation_is_an_explicit_error_and_does_not_replace_the_owner() {
    block_on(async {
        let first = dispatch_handle().unwrap();
        let failure = block_on(async {}).unwrap_err();
        assert_eq!(
            failure.source_error().message(),
            "A JavaScript event loop is already running on this thread."
        );
        let second = dispatch_handle().unwrap();
        first.wake().unwrap();
        second.wake().unwrap();
    })
    .unwrap();
    assert!(dispatch_handle().is_err());
}

#[test]
fn panic_shutdown_releases_frames_and_repeated_loop_use_has_no_stale_jobs() {
    let capture = Rc::new(Cell::new(0));
    let observed = Rc::clone(&capture);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        block_on(async move {
            crate::promise_jobs::enqueue(async move {
                pending::<()>().await;
                observed.set(1);
            })
            .unwrap();
            panic!("original root failure");
        })
    }));
    assert!(failure.is_err());
    assert_eq!(capture.get(), 0);
    assert_eq!(Rc::strong_count(&capture), 1);
    assert!(!crate::promise_jobs::active());
    assert_eq!(block_on(async { 17 }).unwrap(), 17);
    assert_eq!(capture.get(), 0);
}

#[test]
fn job_panic_shutdown_releases_the_remaining_non_send_captures() {
    let capture = Rc::new(Cell::new(0));
    let observed = Rc::clone(&capture);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        block_on(async move {
            crate::promise_jobs::enqueue(async { panic!("original reaction failure") }).unwrap();
            crate::promise_jobs::enqueue(async move { observed.set(1) }).unwrap();
        })
    }));
    assert!(failure.is_err());
    assert_eq!(capture.get(), 0);
    assert_eq!(Rc::strong_count(&capture), 1);
    assert_eq!(block_on(async { 19 }).unwrap(), 19);
}

struct FailingDriver {
    error: Option<JsError>,
}

impl EventLoopDriver for FailingDriver {
    type Error = JsError;

    fn admission_error(error: JsError) -> Self::Error {
        error
    }
    fn lifecycle(&mut self, _transition: LoopLifecycle) -> Result<(), JsError> {
        Ok(())
    }
    fn poll(&mut self) -> Result<bool, JsError> {
        Ok(false)
    }
    fn has_work(&self) -> bool {
        false
    }
    fn wait(&mut self) -> Result<(), JsError> {
        Err(self.error.take().unwrap())
    }
    fn waker(&mut self) -> Result<Waker, JsError> {
        Ok(Waker::noop().clone())
    }
}

#[test]
fn driver_error_retains_identity_and_cancels_unfinished_jobs() {
    let original = JsError::error("original native wait error");
    let capture = Rc::new(Cell::new(0));
    let observed = Rc::clone(&capture);
    let root = async move {
        crate::promise_jobs::enqueue(async move {
            pending::<()>().await;
            observed.set(1);
        })
        .unwrap();
        pending::<()>().await;
    };
    let failure = block_on_with_driver(
        root,
        &mut FailingDriver {
            error: Some(original.clone()),
        },
    )
    .unwrap_err();
    assert!(failure.has_same_identity(&original));
    assert_eq!(capture.get(), 0);
    assert_eq!(Rc::strong_count(&capture), 1);
    assert!(!crate::promise_jobs::active());
}

struct LifecycleDriver<'scope> {
    transitions: &'scope RefCell<Vec<(LoopLifecycle, bool)>>,
    fail_enter: bool,
    fail_wait: bool,
}

impl EventLoopDriver for LifecycleDriver<'_> {
    type Error = JsError;

    fn admission_error(error: JsError) -> JsError {
        error
    }
    fn lifecycle(&mut self, transition: LoopLifecycle) -> Result<(), JsError> {
        self.transitions
            .borrow_mut()
            .push((transition, ensure_running().is_ok()));
        if self.fail_enter && matches!(transition, LoopLifecycle::Enter(_)) {
            return Err(JsError::error("original entry error"));
        }
        Ok(())
    }
    fn poll(&mut self) -> Result<bool, JsError> {
        Ok(false)
    }
    fn has_work(&self) -> bool {
        false
    }
    fn wait(&mut self) -> Result<(), JsError> {
        assert!(self.fail_wait);
        Err(JsError::error("original wait error"))
    }
    fn waker(&mut self) -> Result<Waker, JsError> {
        Ok(Waker::noop().clone())
    }
}

fn assert_lifecycle(transitions: &RefCell<Vec<(LoopLifecycle, bool)>>) {
    let transitions = transitions.borrow();
    assert_eq!(transitions.len(), 2);
    let (LoopLifecycle::Enter(owner), true) = transitions[0] else {
        panic!("activation must precede native registration")
    };
    assert_eq!(transitions[1], (LoopLifecycle::Shutdown(owner), false));
    assert!(!crate::promise_jobs::active());
}

#[test]
fn lifecycle_is_exactly_once_with_admission_closed_on_every_exit_path() {
    for failure in 0..4 {
        let transitions = RefCell::new(Vec::new());
        let mut driver = LifecycleDriver {
            transitions: &transitions,
            fail_enter: failure == 1,
            fail_wait: failure == 2,
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            block_on_with_driver(
                async {
                    if failure == 2 {
                        pending::<()>().await;
                    }
                    assert_ne!(failure, 3, "original root panic");
                },
                &mut driver,
            )
        }));
        match failure {
            0 => assert!(result.unwrap().is_ok()),
            1 => assert_eq!(
                result.unwrap().unwrap_err().message(),
                "original entry error"
            ),
            2 => assert_eq!(
                result.unwrap().unwrap_err().message(),
                "original wait error"
            ),
            3 => assert!(result.is_err()),
            _ => unreachable!(),
        }
        assert_lifecycle(&transitions);
    }
}

#[test]
fn denied_native_context_entry_never_closes_another_live_owner() {
    use tsonic_rust_runtime::dispatch::{prepend, DispatchContexts, DispatchEnd, DispatchOwner};
    let context = crate::timers::new::<tsonic_rust_runtime::TsonicError>();
    let other = DispatchOwner::new().unwrap();
    assert!(context.enter(other));
    let capture = Rc::new(Cell::new(false));
    let retained = Rc::clone(&capture);
    let handle = context
        .schedule_with(
            std::time::Duration::from_secs(60),
            false,
            true,
            false,
            || {
                Callable::new(move |()| {
                    retained.set(true);
                    Ok(())
                })
            },
        )
        .unwrap();
    for _ in 0..128 {
        let contexts = prepend(
            &context,
            DispatchEnd::<tsonic_rust_runtime::TsonicError>::new(),
        );
        assert!(block_on_with_contexts(async {}, contexts).is_err());
        assert!(context.belongs_to(other));
        assert!(context.has_pending_work());
        assert!(handle.has_ref());
        assert_eq!(Rc::strong_count(&capture), 2);
    }
    context.close(other);
    assert!(!handle.has_ref());
    assert_eq!(Rc::strong_count(&capture), 1);
    assert!(!capture.get());
}

#[test]
fn distinct_loop_threads_never_clear_each_others_native_resources() {
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let (completed, completion) = std::sync::mpsc::sync_channel(1);
    let first_barrier = Arc::clone(&barrier);
    let first = std::thread::spawn(move || {
        block_on(async {
            first_barrier.wait();
        })
        .unwrap();
        completed.send(()).unwrap();
    });
    let second = std::thread::spawn(move || {
        let called = Rc::new(Cell::new(0));
        let retained = Rc::clone(&called);
        block_on(async {
            let owner = dispatch_owner().unwrap();
            crate::timers::with_default(|timers| {
                crate::timers::set_timeout_callable(
                    timers,
                    Callable::new(move |()| {
                        retained.set(1);
                        Ok(())
                    }),
                    0.0,
                )
                .unwrap();
            });
            barrier.wait();
            completion.recv().unwrap();
            crate::timers::with_default(|timers| {
                assert!(timers.belongs_to(owner));
                assert!(timers.has_pending_work());
            });
        })
        .unwrap();
        assert_eq!(called.get(), 1);
        assert_eq!(Rc::strong_count(&called), 1);
    });
    first.join().unwrap();
    second.join().unwrap();
}

#[test]
fn copied_owners_and_precaptured_native_handles_remain_generation_bound() {
    use tsonic_rust_runtime::dispatch::{prepend, DispatchContexts, DispatchEnd};
    use tsonic_rust_runtime::dispatch_queue::{TaskBudget, TaskQueue, TaskQueueError};
    let queue = TaskQueue::<tsonic_rust_runtime::TsonicError>::new_closed(TaskBudget::new(
        std::num::NonZeroUsize::new(2).unwrap(),
    ));
    struct QueueContext<'queue>(&'queue TaskQueue<tsonic_rust_runtime::TsonicError>);
    impl DispatchContexts for QueueContext<'_> {
        type Error = tsonic_rust_runtime::TsonicError;
        type Frontier = ();
        fn enter(&self, owner: DispatchOwner) -> bool {
            self.0.enter(owner)
        }
        fn close(&self, owner: DispatchOwner) {
            self.0.close(owner);
        }
        fn prepare(
            &self,
            _phase: tsonic_rust_runtime::dispatch::DispatchPhase,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
        fn next_ready(&self, _frontier: &()) -> Option<u64> {
            None
        }
        fn poll_next(&self, _frontier: &()) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn has_work(&self) -> bool {
            false
        }
        fn next_delay(&self) -> Option<std::time::Duration> {
            None
        }
    }
    let first_handle = queue.handle();
    let (retired, status) = block_on_with_contexts(
        async { dispatch_owner().unwrap() },
        prepend(
            &QueueContext(&queue),
            DispatchEnd::<tsonic_rust_runtime::TsonicError>::new(),
        ),
    )
    .unwrap();
    assert!(!status.exit_requested());
    let retired = retired.expect("the root completes before the loop exits");
    for _ in 0..128 {
        let next = queue.handle();
        assert_eq!(next.enqueue(|| Ok(())).unwrap_err(), TaskQueueError::Closed);
        let context = QueueContext(&queue);
        block_on_with_contexts(
            async {
                let current = dispatch_owner().unwrap();
                assert_ne!(current, retired);
                assert!(!queue.enter(retired));
                queue.close(retired);
                next.enqueue(|| Ok(())).unwrap();
                assert_eq!(
                    first_handle.enqueue(|| Ok(())).unwrap_err(),
                    TaskQueueError::Closed
                );
            },
            prepend(
                &context,
                DispatchEnd::<tsonic_rust_runtime::TsonicError>::new(),
            ),
        )
        .unwrap();
    }
}

struct ProgressDriver<'scope> {
    ready: &'scope Cell<bool>,
    dispatched: &'scope Cell<usize>,
    polls: usize,
    waits: usize,
    failure: JsError,
}

impl EventLoopDriver for ProgressDriver<'_> {
    type Error = JsError;
    fn admission_error(error: JsError) -> JsError {
        error
    }
    fn lifecycle(&mut self, _transition: LoopLifecycle) -> Result<(), JsError> {
        Ok(())
    }
    fn poll(&mut self) -> Result<bool, JsError> {
        self.polls += 1;
        let ready = self.ready.replace(false);
        if ready {
            self.dispatched.set(self.dispatched.get() + 1);
        }
        Ok(ready)
    }
    fn has_work(&self) -> bool {
        self.ready.get()
    }
    fn wait(&mut self) -> Result<(), JsError> {
        self.waits += 1;
        Err(self.failure.clone())
    }
    fn waker(&mut self) -> Result<Waker, JsError> {
        Ok(Waker::noop().clone())
    }
}

#[test]
fn newly_admitted_native_work_gets_a_dispatch_turn_before_waiting() {
    let ready = Cell::new(false);
    let dispatched = Cell::new(0);
    let mut driver = ProgressDriver {
        ready: &ready,
        dispatched: &dispatched,
        polls: 0,
        waits: 0,
        failure: JsError::error("premature wait"),
    };
    let (output, status) = block_on_with_driver(
        async {
            ready.set(true);
            7
        },
        &mut driver,
    )
    .unwrap();
    assert_eq!(output, Some(7));
    assert!(!status.exit_requested());
    assert_eq!(dispatched.get(), 1);
    assert_eq!(driver.polls, 1);
    assert_eq!(driver.waits, 0);
}

struct RootPrefixDriver<'scope> {
    subscribed: &'scope Cell<bool>,
    delivered: &'scope Cell<bool>,
    pending: bool,
    referenced: bool,
    polls: usize,
}

impl EventLoopDriver for RootPrefixDriver<'_> {
    type Error = JsError;
    fn admission_error(error: JsError) -> JsError {
        error
    }
    fn lifecycle(&mut self, _transition: LoopLifecycle) -> Result<(), JsError> {
        Ok(())
    }
    fn poll(&mut self) -> Result<bool, JsError> {
        self.polls += 1;
        let pending = std::mem::replace(&mut self.pending, false);
        if pending {
            self.delivered.set(self.subscribed.get());
        }
        Ok(pending)
    }
    fn has_work(&self) -> bool {
        self.referenced && self.pending
    }
    fn wait(&mut self) -> Result<(), JsError> {
        Err(JsError::error("the completed root must never wait"))
    }
    fn waker(&mut self) -> Result<Waker, JsError> {
        Ok(Waker::noop().clone())
    }
}

#[test]
fn the_async_root_subscribes_before_retained_native_work_is_dispatched() {
    let subscribed = Cell::new(false);
    let delivered = Cell::new(false);
    let root_polls = Cell::new(0);
    let mut driver = RootPrefixDriver {
        subscribed: &subscribed,
        delivered: &delivered,
        pending: true,
        referenced: true,
        polls: 0,
    };
    let (output, status) = block_on_with_driver(
        poll_fn(|_| {
            root_polls.set(root_polls.get() + 1);
            subscribed.set(true);
            Poll::Ready(11)
        }),
        &mut driver,
    )
    .unwrap();
    assert_eq!(output, Some(11));
    assert!(!status.exit_requested());
    assert!(delivered.get());
    assert_eq!(root_polls.get(), 1);
    assert_eq!(driver.polls, 1);
}

#[test]
fn a_ready_root_retires_without_dispatching_unreferenced_native_work() {
    let subscribed = Cell::new(false);
    let delivered = Cell::new(false);
    let root_polls = Cell::new(0);
    let mut driver = RootPrefixDriver {
        subscribed: &subscribed,
        delivered: &delivered,
        pending: true,
        referenced: false,
        polls: 0,
    };
    let (output, status) = block_on_with_driver(
        poll_fn(|_| {
            root_polls.set(root_polls.get() + 1);
            subscribed.set(true);
            Poll::Ready(17)
        }),
        &mut driver,
    )
    .unwrap();
    assert_eq!(output, Some(17));
    assert!(!status.exit_requested());
    assert!(!delivered.get());
    assert!(driver.pending);
    assert_eq!(root_polls.get(), 1);
    assert_eq!(driver.polls, 0);
}

#[test]
fn a_pending_non_self_waking_root_reaches_wait_without_repeated_polling() {
    let ready = Cell::new(false);
    let dispatched = Cell::new(0);
    let root_polls = Cell::new(0);
    let failure = JsError::error("original native wait");
    let mut driver = ProgressDriver {
        ready: &ready,
        dispatched: &dispatched,
        polls: 0,
        waits: 0,
        failure: failure.clone(),
    };
    let observed = block_on_with_driver(
        poll_fn(|_| {
            root_polls.set(root_polls.get() + 1);
            Poll::<()>::Pending
        }),
        &mut driver,
    )
    .unwrap_err();
    assert!(observed.has_same_identity(&failure));
    assert_eq!(root_polls.get(), 1);
    assert_eq!(driver.polls, 2);
    assert_eq!(driver.waits, 1);
    assert_eq!(dispatched.get(), 0);
}
