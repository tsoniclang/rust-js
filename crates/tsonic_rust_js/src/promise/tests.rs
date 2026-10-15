use super::*;
use crate::equality::JsStrictEqual;
use crate::event_loop::{block_on, block_on_with_contexts, run_entry_with_contexts};
use crate::{JsArray, JsValue};
use std::cell::Cell;
use std::future::{poll_fn, Future};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use tsonic_rust_runtime::dispatch::DispatchEnd;
use tsonic_rust_runtime::{
    Callable, ErrorObject, JsError, JsErrorKind, MutableJsError, TsonicError, WritableErrorObject,
};

const NO_LOOP: &str = "No JavaScript event loop is running on this thread.";

fn no_contexts() -> DispatchEnd<TsonicError> {
    DispatchEnd::<TsonicError>::new()
}

#[test]
fn settled_payloads_keep_native_borrows_and_consume_without_copying() {
    let text = String::from("borrowed payload");
    let promise: JsPromise<&str> = JsPromise::resolved(text.as_str());
    let mut result = std::pin::pin!(promise.into_result());
    assert!(matches!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Ok(value)) if value == text.as_str()
    ));
}

#[test]
fn settled_constructors_need_no_owner_and_admit_no_job() {
    assert!(!crate::promise_jobs::active());
    let fulfilled = JsPromise::from_result(Ok::<i32, JsValue>(5));
    let rejected = JsPromise::<i32>::from_result(Err(JsValue::Bool(true)));
    let mut fulfilled = std::pin::pin!(fulfilled.into_result());
    let mut rejected = std::pin::pin!(rejected.into_result());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(
        fulfilled.as_mut().poll(&mut context),
        Poll::Ready(Ok(5))
    ));
    assert!(matches!(
        rejected.as_mut().poll(&mut context),
        Poll::Ready(Err(JsValue::Bool(true)))
    ));
    assert!(!crate::promise_jobs::active());
}

#[test]
fn rejection_accepts_the_closed_value_domain_without_a_second_error_type() {
    for reason in [
        JsValue::null(),
        JsValue::Bool(false),
        JsValue::UnsignedInteger(u64::MAX),
        JsValue::String(String::from("owned reason")),
        JsValue::from_error(JsError::error("native")),
    ] {
        let expected = reason.clone();
        let result = block_on(JsPromise::<i32>::rejected(reason).into_result())
            .unwrap()
            .unwrap_err();
        assert!(result.strict_equal(&expected));
    }
}

#[test]
fn retained_errors_keep_identity_and_writable_fields_through_a_package_boundary() {
    fn exported_failure(reason: JsValue) -> JsPromise<i32> {
        JsPromise::rejected(reason)
    }
    let authored = MutableJsError::error("before");
    let original = JsValue::from_error(authored.clone());
    let promise = exported_failure(original.clone());
    authored.set_error_message(String::from("after"));
    let error = block_on(promise.into_result()).unwrap().unwrap_err();
    assert!(error.strict_equal(&original));
    assert_eq!(error.error_value().error_message(), "after");
}

#[test]
fn a_factory_runs_its_body_eagerly_to_the_first_pending_point() {
    block_on(async {
        let effects = Rc::new(Cell::new(0));
        let body = Rc::clone(&effects);
        let ready = Rc::new(Cell::new(false));
        let gate = Rc::clone(&ready);
        let promise = JsPromise::from_infallible_factory(move || async move {
            body.set(1);
            poll_fn(|context| {
                if gate.get() {
                    Poll::Ready(())
                } else {
                    context.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
            body.set(2);
            11
        });
        assert_eq!(
            effects.get(),
            1,
            "the prefix ran before the factory returned"
        );
        ready.set(true);
        assert_eq!(promise.into_result().await.unwrap(), 11);
        assert_eq!(effects.get(), 2);
    })
    .unwrap();
}

#[test]
fn a_synchronous_body_settles_before_the_factory_returns() {
    block_on(async {
        let promise = JsPromise::<i32>::from_fallible_factory(|| async {
            Err(JsValue::String(String::from("synchronous rejection")))
        });
        let mut settled = std::pin::pin!(promise.await_result());
        assert!(matches!(
            settled
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Err(JsValue::String(text))) if text == "synchronous rejection"
        ));
    })
    .unwrap();
}

#[test]
fn a_factory_without_an_owner_calls_no_create_and_rejects_with_the_exact_message() {
    let created = Cell::new(false);
    let promise = JsPromise::from_infallible_factory(|| {
        created.set(true);
        async { 7 }
    });
    assert!(!created.get(), "no Create body runs without an owner");
    assert!(!crate::promise_jobs::active());
    let error = tsonic_rust_runtime::block_on(promise.into_result()).unwrap_err();
    assert_eq!(error.error_value().error_message(), NO_LOOP);
}

#[test]
fn a_pending_body_keeps_running_after_its_handle_is_dropped() {
    let completed = Rc::new(Cell::new(0));
    let observed = Rc::clone(&completed);
    let (_, status) = run_entry_with_contexts(
        move || {
            let promise = JsPromise::from_infallible_factory(move || async move {
                let timer = JsPromise::create(Callable::new(
                    |(resolve, _): (PromiseResolve<()>, PromiseReject)| {
                        crate::timers::with_default(|timers| {
                            crate::timers::set_timeout_callable(
                                timers,
                                Callable::new(move |()| {
                                    resolve
                                        .call((PromiseResolution::Value(()),))
                                        .expect("native Promise resolution is infallible");
                                    Ok(())
                                }),
                                1.0,
                            )
                        })
                        .expect("timer registration under the owner");
                        Ok(())
                    },
                ));
                timer.into_result().await.unwrap();
                observed.set(41);
            });
            drop(promise);
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!status.exit_requested());
    assert_eq!(completed.get(), 41);
}

#[test]
fn canceling_the_first_waiter_keeps_later_waiters() {
    block_on(async {
        let promise = JsPromise::<i32>::pending();
        let first_wakes = Arc::new(WakeCount::default());
        let second_wakes = Arc::new(WakeCount::default());
        let third_wakes = Arc::new(WakeCount::default());
        let first_waker = Waker::from(Arc::clone(&first_wakes));
        let second_waker = Waker::from(Arc::clone(&second_wakes));
        let third_waker = Waker::from(Arc::clone(&third_wakes));
        let mut first = Box::pin(promise.await_result());
        let mut second = Box::pin(promise.await_result());
        assert!(first
            .as_mut()
            .poll(&mut Context::from_waker(&first_waker))
            .is_pending());
        assert!(second
            .as_mut()
            .poll(&mut Context::from_waker(&second_waker))
            .is_pending());
        drop(first);
        let mut third = Box::pin(promise.await_result());
        assert!(third
            .as_mut()
            .poll(&mut Context::from_waker(&third_waker))
            .is_pending());
        promise.settle(Ok(19));
        assert_eq!(first_wakes.count(), 0);
        assert_eq!(second_wakes.count(), 1);
        assert_eq!(third_wakes.count(), 1);
        assert!(matches!(
            second
                .as_mut()
                .poll(&mut Context::from_waker(&second_waker)),
            Poll::Ready(Ok(19))
        ));
        assert!(matches!(
            third.as_mut().poll(&mut Context::from_waker(&third_waker)),
            Poll::Ready(Ok(19))
        ));
    })
    .unwrap();
}

#[test]
fn canceling_a_non_first_waiter_wakes_only_the_remaining_waiters() {
    block_on(async {
        let promise = JsPromise::<i32>::pending();
        let wakes: Vec<_> = (0..6).map(|_| Arc::new(WakeCount::default())).collect();
        let wakers: Vec<_> = wakes
            .iter()
            .map(|count| Waker::from(Arc::clone(count)))
            .collect();
        let mut waiting: Vec<_> = (0..6)
            .map(|_| Some(Box::pin(promise.await_result())))
            .collect();
        for (index, future) in waiting.iter_mut().enumerate() {
            assert!(future
                .as_mut()
                .unwrap()
                .as_mut()
                .poll(&mut Context::from_waker(&wakers[index]))
                .is_pending());
        }
        waiting[2] = None;
        waiting[5] = None;
        promise.settle(Ok(3));
        let woken: Vec<_> = wakes.iter().map(|count| count.count()).collect();
        assert_eq!(woken, [1, 1, 0, 1, 1, 0]);
    })
    .unwrap();
}

struct OrderedWake {
    id: usize,
    log: Arc<Mutex<Vec<usize>>>,
}

impl std::task::Wake for OrderedWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.log.lock().unwrap().push(self.id);
    }
}

fn wake_order(registered: usize, cancelled: &[usize], later: usize) -> Vec<usize> {
    let promise = JsPromise::<i32>::pending();
    let log = Arc::new(Mutex::new(Vec::new()));
    let register = |id: usize| {
        let waker = Waker::from(Arc::new(OrderedWake {
            id,
            log: Arc::clone(&log),
        }));
        let mut future = Box::pin(promise.await_result());
        assert!(future
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending());
        future
    };
    let mut waiting: Vec<_> = (1..=registered).map(|id| Some(register(id))).collect();
    for &id in cancelled {
        waiting[id - 1] = None;
    }
    waiting.extend((registered + 1..=registered + later).map(|id| Some(register(id))));
    promise.settle(Ok(1));
    drop(waiting);
    let order = log.lock().unwrap().clone();
    order
}

fn assert_wake_order(actual: Vec<usize>, expected: &[usize]) {
    assert!(
        actual == expected,
        "woke {actual:?}; the live waiters registered as {expected:?}"
    );
}

#[test]
fn waiters_wake_in_registration_order_after_the_first_waiter_is_cancelled() {
    assert_wake_order(wake_order(2, &[1], 1), &[2, 3]);
}

#[test]
fn waiters_wake_in_registration_order_after_an_inline_waiter_is_cancelled() {
    assert_wake_order(wake_order(4, &[3], 2), &[1, 2, 4, 5, 6]);
}

#[test]
fn waiters_wake_in_registration_order_after_a_spilled_waiter_is_cancelled() {
    assert_wake_order(wake_order(7, &[6], 3), &[1, 2, 3, 4, 5, 7, 8, 9, 10]);
}

#[test]
fn waiters_wake_in_registration_order_after_several_cancellations() {
    assert_wake_order(wake_order(6, &[1, 3, 5], 3), &[2, 4, 6, 7, 8, 9]);
}

#[test]
fn a_waiter_registered_after_every_earlier_waiter_left_wakes_first() {
    assert_wake_order(wake_order(3, &[1, 2, 3], 2), &[4, 5]);
}

#[test]
fn adoption_takes_the_first_resolution_and_ignores_later_settlements() {
    block_on(async {
        let resolver = Rc::new(RefCell::new(None::<PromiseResolve<u64>>));
        let captured = Rc::clone(&resolver);
        let inner = JsPromise::create(Callable::new(move |(resolve, _)| {
            captured.replace(Some(resolve));
            Ok(())
        }));
        let outer = JsPromise::create(Callable::new(
            move |(resolve, reject): (PromiseResolve<u64>, PromiseReject)| {
                resolve.call((PromiseResolution::Promise(inner.clone()),))?;
                reject.call((JsValue::from_error(JsError::error("late rejection")),))?;
                resolve.call((PromiseResolution::Value(0),))?;
                Err(JsValue::from_error(JsError::error("late throw")))
            },
        ));
        resolver
            .borrow()
            .as_ref()
            .unwrap()
            .call((PromiseResolution::Value(9_007_199_254_740_993),))
            .unwrap();
        assert_eq!(outer.into_result().await.unwrap(), 9_007_199_254_740_993);
    })
    .unwrap();
}

#[test]
fn self_resolution_rejects_with_a_type_error() {
    block_on(async {
        let resolver = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
        let captured = Rc::clone(&resolver);
        let promise = JsPromise::create(Callable::new(move |(resolve, _)| {
            captured.replace(Some(resolve));
            Ok(())
        }));
        resolver
            .borrow_mut()
            .take()
            .unwrap()
            .call((PromiseResolution::Promise(promise.clone()),))
            .unwrap();
        let error = promise.into_result().await.unwrap_err();
        assert!(error.is_error_kind(JsErrorKind::TypeError));
    })
    .unwrap();
}

#[test]
fn unique_ready_and_pending_consumes_move_the_payload() {
    block_on(async {
        let clones = Rc::new(Cell::new(0));
        let ready = JsPromise::resolved(CountedClone(Rc::clone(&clones)));
        let _moved = ready.into_result().await.unwrap();
        let pending_clones = Rc::clone(&clones);
        let pending = JsPromise::from_infallible_factory(move || async move {
            crate::promise_jobs::enqueue(async {}).unwrap();
            yield_once().await;
            CountedClone(pending_clones)
        });
        let _moved = pending.into_result().await.unwrap();
        assert_eq!(clones.get(), 0);
    })
    .unwrap();
}

#[test]
fn shared_observations_repeat_and_clone_the_payload() {
    block_on(async {
        let promise = JsPromise::resolved(String::from("shared"));
        assert_eq!(promise.await_result().await.unwrap(), "shared");
        assert_eq!(promise.await_result().await.unwrap(), "shared");
        assert_eq!(promise.clone().into_result().await.unwrap(), "shared");
        assert_eq!(promise.into_result().await.unwrap(), "shared");
    })
    .unwrap();
}

#[test]
fn a_discarded_fulfilled_promise_admits_no_job_and_requests_no_exit() {
    let (_, status) = run_entry_with_contexts(
        || {
            JsPromise::resolved(1).observe_discarded();
            assert!(!crate::promise_jobs::has_ready());
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!status.exit_requested());
}

#[test]
fn a_discarded_rejection_reports_after_the_current_job_and_requests_exit_one() {
    let later = Rc::new(Cell::new(false));
    let observed = Rc::clone(&later);
    let (_, status) = run_entry_with_contexts(
        move || {
            JsPromise::<i32>::rejected(JsValue::String(String::from("discarded")))
                .observe_discarded();
            crate::promise_jobs::enqueue(async move { observed.set(true) }).unwrap();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(status.exit_requested());
    assert_eq!(
        std::process::ExitCode::from(status),
        std::process::ExitCode::from(1)
    );
    assert!(!later.get(), "the loop stops after the reporting job");
    assert_eq!(take_reports(), "discarded\n");
}

struct DiscardOnDrop(&'static str);

impl Drop for DiscardOnDrop {
    fn drop(&mut self) {
        JsPromise::<i32>::rejected(JsValue::String(String::from(self.0))).observe_discarded();
    }
}

#[test]
fn a_rejection_observed_while_the_owner_closes_is_written_and_requests_exit_one() {
    let ((), status) = run_entry_with_contexts(
        || {
            drop(JsPromise::<()>::from_infallible_factory(|| async {
                let _guard = DiscardOnDrop("closing");
                std::future::pending::<()>().await;
            }));
        },
        no_contexts(),
    )
    .unwrap();
    assert!(status.exit_requested());
    assert_eq!(
        std::process::ExitCode::from(status),
        std::process::ExitCode::from(1)
    );
    assert_eq!(take_reports(), "closing\n");
}

#[test]
fn the_completion_state_of_an_i32_promise_is_forty_eight_bytes() {
    println!(
        "promise-allocation row=state size_of_promise_state_i32={} size_of_refcell_state_i32={}",
        std::mem::size_of::<PromiseState<i32>>(),
        std::mem::size_of::<RefCell<PromiseState<i32>>>()
    );
    assert_eq!(std::mem::size_of::<PromiseState<i32>>(), 48);
    assert_eq!(std::mem::size_of::<RefCell<PromiseState<i32>>>(), 56);
}

#[test]
fn a_pending_discarded_promise_reports_only_when_it_rejects() {
    let (_, fulfilled) = run_entry_with_contexts(
        || {
            let promise = JsPromise::<i32>::from_infallible_factory(|| async {
                yield_once().await;
                3
            });
            promise.observe_discarded();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!fulfilled.exit_requested());
    let (_, rejected) = run_entry_with_contexts(
        || {
            let promise = JsPromise::<i32>::from_fallible_factory(|| async {
                yield_once().await;
                Err(JsValue::from_error(JsError::error("late failure")))
            });
            promise.observe_discarded();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(rejected.exit_requested());
}

#[test]
fn a_stored_unobserved_rejection_is_not_reported() {
    let stored = Rc::new(RefCell::new(None));
    let retained = Rc::clone(&stored);
    let (_, status) = run_entry_with_contexts(
        move || {
            retained.replace(Some(JsPromise::<i32>::rejected(JsValue::Bool(false))));
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!status.exit_requested());
    assert!(stored.borrow().is_some());
}

fn zero_delay_timer(ran: Rc<Cell<bool>>) {
    crate::timers::with_default(|timers| {
        crate::timers::set_timeout_callable(
            timers,
            Callable::new(move |()| {
                ran.set(true);
                Ok(())
            }),
            0.0,
        )
    })
    .unwrap();
}

fn discard_boom() {
    JsPromise::<i32>::from_fallible_factory(|| async {
        Err(JsValue::from_error(JsError::error("boom")))
    })
    .observe_discarded();
}

#[test]
fn a_requested_exit_from_a_synchronous_entry_stops_before_a_zero_delay_timer() {
    let timer_ran = Rc::new(Cell::new(false));
    let observed = Rc::clone(&timer_ran);
    let ((), status) = run_entry_with_contexts(
        move || {
            zero_delay_timer(observed);
            discard_boom();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(status.exit_requested());
    assert!(
        !timer_ran.get(),
        "no loop work restarts after the exit request"
    );
}

#[test]
fn a_requested_exit_from_an_asynchronous_entry_stops_before_a_zero_delay_timer() {
    let timer_ran = Rc::new(Cell::new(false));
    let observed = Rc::clone(&timer_ran);
    let (output, status) = block_on_with_contexts(
        async move {
            zero_delay_timer(observed);
            discard_boom();
            9
        },
        no_contexts(),
    )
    .unwrap();
    assert_eq!(output, Some(9));
    assert!(status.exit_requested());
    assert!(
        !timer_ran.get(),
        "no loop work restarts after the exit request"
    );
}

#[test]
fn without_a_discarded_rejection_the_zero_delay_timer_runs() {
    let timer_ran = Rc::new(Cell::new(false));
    let observed = Rc::clone(&timer_ran);
    let ((), status) =
        run_entry_with_contexts(move || zero_delay_timer(observed), no_contexts()).unwrap();
    assert!(!status.exit_requested());
    assert!(timer_ran.get());
}

#[test]
fn a_requested_exit_drops_a_pending_root_and_reports_no_output() {
    let (output, status) = block_on_with_contexts(
        async {
            JsPromise::<i32>::rejected(JsValue::Bool(false)).observe_discarded();
            std::future::pending::<i32>().await
        },
        no_contexts(),
    )
    .unwrap();
    assert!(output.is_none());
    assert!(status.exit_requested());
    let error = block_on(async {
        JsPromise::<i32>::rejected(JsValue::Bool(false)).observe_discarded();
        std::future::pending::<i32>().await
    })
    .unwrap_err();
    assert_eq!(
        error.source_error().message(),
        "The event loop stopped on a requested exit before the awaited future completed."
    );
}

#[test]
fn all_preserves_input_order_and_exact_integer_payloads() {
    block_on(async {
        let values = JsArray::from_dense(vec![
            JsPromise::resolved(u64::MAX),
            JsPromise::resolved(9_007_199_254_740_993),
        ]);
        let result = promise_all(&values).into_result().await.unwrap();
        assert_eq!(result.values(), [u64::MAX, 9_007_199_254_740_993]);
    })
    .unwrap();
}

#[test]
fn all_propagates_the_original_rejection_without_error_conversion() {
    block_on(async {
        let error = JsValue::from_error(JsError::error("all failure"));
        let values = JsArray::from_dense(vec![
            JsPromise::resolved(1),
            JsPromise::rejected(error.clone()),
        ]);
        let actual = promise_all(&values).into_result().await.unwrap_err();
        assert!(actual.strict_equal(&error));
    })
    .unwrap();
}

#[test]
fn all_and_all_settled_empty_inputs_are_already_fulfilled() {
    let values = JsArray::<JsPromise<i32>>::new();
    let all = promise_all(&values);
    let settled = promise_all_settled(&values);
    let mut all = std::pin::pin!(all.into_result());
    let mut settled = std::pin::pin!(settled.into_result());
    let mut context = Context::from_waker(Waker::noop());
    assert!(
        matches!(all.as_mut().poll(&mut context), Poll::Ready(Ok(values)) if values.is_empty())
    );
    assert!(
        matches!(settled.as_mut().poll(&mut context), Poll::Ready(Ok(values)) if values.is_empty())
    );
}

#[test]
fn race_empty_input_is_pending_without_acquiring_a_loop() {
    let values = JsArray::<JsPromise<i32>>::new();
    let race = promise_race(&values);
    let mut race = std::pin::pin!(race.into_result());
    assert!(race
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending());
    assert!(!crate::promise_jobs::active());
}

#[test]
fn race_and_any_choose_first_settlement_and_first_fulfillment() {
    block_on(async {
        let reason = JsValue::String(String::from("first rejection"));
        let values = JsArray::from_dense(vec![
            JsPromise::rejected(reason.clone()),
            JsPromise::resolved(7),
            JsPromise::resolved(9),
        ]);
        assert!(promise_race(&values)
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
        assert_eq!(promise_any(&values).into_result().await.unwrap(), 7);
    })
    .unwrap();
}

#[test]
fn any_uses_the_supported_aggregate_error_for_empty_and_all_rejected_inputs() {
    block_on(async {
        for values in [
            JsArray::<JsPromise<i32>>::new(),
            JsArray::from_dense(vec![JsPromise::rejected(JsValue::Bool(false))]),
        ] {
            let error = promise_any(&values).into_result().await.unwrap_err();
            assert!(error.is_error_kind(JsErrorKind::AggregateError));
            assert_eq!(
                error.error_value().error_message(),
                "All promises were rejected"
            );
        }
    })
    .unwrap();
}

#[test]
fn all_settled_preserves_order_and_original_reasons() {
    block_on(async {
        let error = JsValue::from_error(JsError::error("settled failure"));
        let values = JsArray::from_dense(vec![
            JsPromise::rejected(error.clone()),
            JsPromise::resolved(7),
        ]);
        let result = promise_all_settled(&values).into_result().await.unwrap();
        assert!(
            matches!(result.get(0), Some(PromiseSettledResult::Rejected(rejected))
            if rejected.status == "rejected" && rejected.reason.strict_equal(&error))
        );
        assert!(
            matches!(result.get(1), Some(PromiseSettledResult::Fulfilled(fulfilled))
            if fulfilled.status == "fulfilled" && fulfilled.value == 7)
        );
    })
    .unwrap();
}

#[test]
fn then_catch_and_finally_are_deferred_and_preserve_reason_identity() {
    block_on(async {
        let calls = Rc::new(Cell::new(0));
        let observed = Rc::clone(&calls);
        let reason = JsValue::from_error(JsError::error("callback failure"));
        let thrown = reason.clone();
        let failed = JsPromise::resolved(1).then(
            Callable::new(move |(_value,)| Err::<i32, _>(thrown.clone())),
            None,
        );
        let finalized = failed.finally(Callable::new(move |()| {
            observed.set(observed.get() + 1);
            Ok(())
        }));
        let expected = reason.clone();
        let recovered = finalized.catch(Callable::new(move |(error,): (JsValue,)| {
            assert!(error.strict_equal(&expected));
            Ok(19)
        }));
        assert_eq!(calls.get(), 0);
        assert_eq!(recovered.into_result().await.unwrap(), 19);
        assert_eq!(calls.get(), 1);
        assert!(failed
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
    })
    .unwrap();
}

#[test]
fn a_finally_failure_replaces_the_source_reason_without_calling_success_reactions() {
    block_on(async {
        let reason = JsValue::from_error(JsError::error("cleanup"));
        let thrown = reason.clone();
        let source = JsPromise::resolved(7);
        let result = source.finally(Callable::new(move |()| Err(thrown.clone())));
        assert!(result
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
        assert_eq!(source.into_result().await.unwrap(), 7);
    })
    .unwrap();
}

#[test]
fn ordered_combinators_wait_for_all_inputs_even_when_completion_order_is_reversed() {
    block_on(async {
        let first = JsPromise::pending();
        let second = JsPromise::pending();
        let values = JsArray::from_dense(vec![first.clone(), second.clone()]);
        let all = promise_all(&values);
        let settled = promise_all_settled(&values);
        crate::promise_jobs::enqueue(async move {
            second.settle(Ok(29));
            crate::promise_jobs::enqueue(async move {
                first.settle(Ok(17));
            })
            .unwrap();
        })
        .unwrap();
        assert_eq!(all.into_result().await.unwrap().values(), [17, 29]);
        let settled = settled.into_result().await.unwrap();
        assert!(
            matches!(settled.get(0), Some(PromiseSettledResult::Fulfilled(value)) if value.value == 17)
        );
        assert!(
            matches!(settled.get(1), Some(PromiseSettledResult::Fulfilled(value)) if value.value == 29)
        );
    })
    .unwrap();
}

#[test]
fn all_fails_fast_without_waiting_for_an_unsettled_input() {
    block_on(async {
        let reason = JsValue::from_error(JsError::error("early failure"));
        let values = JsArray::from_dense(vec![
            JsPromise::<i32>::pending(),
            JsPromise::rejected(reason.clone()),
        ]);
        assert!(promise_all(&values)
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
    })
    .unwrap();
}

#[test]
fn race_observes_the_first_pending_input_to_settle() {
    block_on(async {
        let first = JsPromise::<i32>::pending();
        let second = JsPromise::pending();
        let values = JsArray::from_dense(vec![first, second.clone()]);
        let result = promise_race(&values);
        let reason = JsValue::String(String::from("second settles first"));
        let thrown = reason.clone();
        crate::promise_jobs::enqueue(async move {
            second.settle(Err(thrown));
        })
        .unwrap();
        assert!(result
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
    })
    .unwrap();
}

#[test]
fn any_waits_for_fulfillment_after_an_earlier_rejection() {
    block_on(async {
        let first = JsPromise::<i32>::pending();
        let second = JsPromise::pending();
        let values = JsArray::from_dense(vec![first.clone(), second.clone()]);
        let result = promise_any(&values);
        crate::promise_jobs::enqueue(async move {
            first.settle(Err(JsValue::Bool(false)));
            crate::promise_jobs::enqueue(async move {
                second.settle(Ok(31));
            })
            .unwrap();
        })
        .unwrap();
        assert_eq!(result.into_result().await.unwrap(), 31);
    })
    .unwrap();
}

#[test]
fn an_exception_in_then_does_not_invoke_that_same_calls_reject_handler() {
    block_on(async {
        let calls = Rc::new(Cell::new(0));
        let rejected = Rc::clone(&calls);
        let reason = JsValue::from_error(JsError::error("then failure"));
        let thrown = reason.clone();
        let result = JsPromise::resolved(7).then(
            Callable::new(move |(_value,)| Err(thrown.clone())),
            Some(Callable::new(move |(_reason,)| {
                rejected.set(1);
                Ok(0)
            })),
        );
        assert_eq!(calls.get(), 0);
        assert!(result
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
        assert_eq!(calls.get(), 0);
    })
    .unwrap();
}

#[test]
fn asynchronous_then_adopts_success_and_failure_without_rewrapping_reasons() {
    block_on(async {
        let source = JsPromise::resolved(7);
        let success = source.then_async(
            Callable::new(|(value,)| Ok(JsPromise::resolved(value * 2))),
            None,
        );
        assert_eq!(success.into_result().await.unwrap(), 14);
        let reason = JsValue::from_error(JsError::error("adopted failure"));
        let thrown = reason.clone();
        let failure = source.then_async(
            Callable::new(move |(_value,)| Ok(JsPromise::<i32>::rejected(thrown.clone()))),
            None,
        );
        assert!(failure
            .into_result()
            .await
            .unwrap_err()
            .strict_equal(&reason));
    })
    .unwrap();
}

#[test]
fn a_reaction_without_an_owner_rejects_and_does_not_become_lazy() {
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let reaction = JsPromise::resolved(1).then(
        Callable::new(move |(_value,)| {
            observed.set(1);
            Ok(2)
        }),
        None,
    );
    let error = tsonic_rust_runtime::block_on(reaction.into_result()).unwrap_err();
    assert_eq!(error.error_value().error_message(), NO_LOOP);
    assert_eq!(calls.get(), 0);
}

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct WakeCount(AtomicUsize);

impl WakeCount {
    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl std::task::Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct CountedClone(Rc<Cell<usize>>);

impl Clone for CountedClone {
    fn clone(&self) -> Self {
        self.0.set(self.0.get() + 1);
        Self(Rc::clone(&self.0))
    }
}

async fn yield_once() {
    let mut yielded = false;
    poll_fn(|context| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}
