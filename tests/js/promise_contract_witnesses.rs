use std::cell::{Cell, RefCell};
use std::future::{poll_fn, Future};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::abi::promise_race;
use tsonic_rust_js::event_loop::block_on;
use tsonic_rust_js::promise::{JsPromise, PromiseReject, PromiseResolution, PromiseResolve};
use tsonic_rust_js::JsArray;
use tsonic_rust_runtime::Callable;

const NO_LOOP: &str = "No JavaScript event loop is running on this thread.";

#[test]
fn a_factory_body_runs_to_its_first_pending_point_before_the_factory_returns() {
    let effects = Rc::new(Cell::new(0));
    let body_effects = Rc::clone(&effects);
    let gate = Rc::new(Cell::new(false));
    let body_gate = Rc::clone(&gate);
    let observed = Rc::clone(&effects);
    let result = block_on(async move {
        let promise: JsPromise<i32> = JsPromise::from_infallible_factory(move || async move {
            body_effects.set(1);
            poll_fn(|context| {
                if body_gate.get() {
                    Poll::Ready(())
                } else {
                    context.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
            body_effects.set(2);
            11
        });
        let prefix = observed.get();
        gate.set(true);
        let value = promise.into_result().await.unwrap();
        (prefix, value)
    })
    .unwrap();
    assert_eq!(
        result.0, 1,
        "the body prefix ran before the factory returned"
    );
    assert_eq!(result.1, 11);
    assert_eq!(effects.get(), 2);
}

#[test]
fn a_pending_body_keeps_running_under_a_real_timer_after_its_handle_is_dropped() {
    let completed = Rc::new(Cell::new(0));
    let observed = Rc::clone(&completed);
    block_on(async move {
        let promise: JsPromise<()> = JsPromise::from_infallible_factory(move || async move {
            let timer: JsPromise<()> = JsPromise::create(Callable::new(
                move |(resolve, _): (PromiseResolve<()>, PromiseReject)| {
                    tsonic_rust_js::timers::with_default(|timers| {
                        tsonic_rust_js::timers::set_timeout_callable(
                            timers,
                            Callable::new(move |()| {
                                let _ = resolve.call((PromiseResolution::Value(()),));
                                Ok(())
                            }),
                            1.0,
                        )
                    })
                    .expect("timer registration under the owner");
                    Ok(())
                },
            ));
            let _ = timer.into_result().await;
            observed.set(41);
        });
        drop(promise);
    })
    .unwrap();
    assert_eq!(
        completed.get(),
        41,
        "the dropped handle did not cancel its body"
    );
}

#[test]
fn a_settled_promise_continues_inline_on_its_first_poll() {
    let promise: JsPromise<i32> = JsPromise::resolved(5);
    let mut result = pin!(promise.into_result());
    assert!(matches!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Ok(5))
    ));
}

#[test]
fn a_factory_without_an_owner_panics_before_create_with_the_exact_message() {
    let created = Rc::new(Cell::new(false));
    let observed = Rc::clone(&created);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        JsPromise::from_infallible_factory(move || {
            observed.set(true);
            async { 7 }
        })
    }))
    .err()
    .expect("owned factory admission must panic synchronously");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied());
    assert_eq!(message, Some(NO_LOOP));
    assert!(!created.get(), "no Create body runs without an owner");
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

#[test]
fn reactions_on_one_promise_run_in_registration_order_after_a_race_drops_its_wait() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&order);
    block_on(async move {
        let p_slot = Rc::new(RefCell::new(None));
        let p_captured = Rc::clone(&p_slot);
        let p: JsPromise<i32> = JsPromise::create(Callable::new(
            move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
                p_captured.replace(Some(resolve));
                Ok(())
            },
        ));
        let q_slot = Rc::new(RefCell::new(None));
        let q_captured = Rc::clone(&q_slot);
        let q: JsPromise<i32> = JsPromise::create(Callable::new(
            move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
                q_captured.replace(Some(resolve));
                Ok(())
            },
        ));
        let resolve_p = p_slot.borrow_mut().take().expect("p's executor ran");
        let resolve_q = q_slot.borrow_mut().take().expect("q's executor ran");
        let raced: JsPromise<i32> = promise_race(&JsArray::from_dense(vec![p.clone(), q.clone()]));
        let mut race_wait = Box::pin(raced.into_result());
        assert!(race_wait
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending());
        let first = Rc::clone(&log);
        let a: JsPromise<i32> = p.then(
            Callable::new(move |(value,): (i32,)| {
                first.borrow_mut().push("A");
                Ok(value)
            }),
            None,
        );
        yield_once().await;
        let _ = resolve_q.call((PromiseResolution::Value(0),));
        yield_once().await;
        let _ = race_wait.await;
        let second = Rc::clone(&log);
        let c: JsPromise<i32> = p.then(
            Callable::new(move |(value,): (i32,)| {
                second.borrow_mut().push("C");
                Ok(value)
            }),
            None,
        );
        yield_once().await;
        let _ = resolve_p.call((PromiseResolution::Value(1),));
        let _ = a.into_result().await;
        let _ = c.into_result().await;
    })
    .unwrap();
    let order = order.borrow();
    assert!(
        *order == ["A", "C"],
        "reactions ran as {order:?}; they were registered as A, C"
    );
}

#[test]
fn a_reaction_on_a_promise_settled_by_a_host_callback_runs_before_the_awaiting_root() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&order);
    block_on(async move {
        let slot = Rc::new(RefCell::new(None));
        let captured = Rc::clone(&slot);
        let p: JsPromise<i32> = JsPromise::create(Callable::new(
            move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
                captured.replace(Some(resolve));
                Ok(())
            },
        ));
        let resolve = slot.borrow_mut().take().expect("p's executor ran");
        let source = p.clone();
        let reaction_log = Rc::clone(&log);
        tsonic_rust_js::timers::with_default(|timers| {
            tsonic_rust_js::timers::set_timeout_callable(
                timers,
                Callable::new(move |()| {
                    let reaction_log = Rc::clone(&reaction_log);
                    let reaction: JsPromise<i32> = source.then(
                        Callable::new(move |(value,): (i32,)| {
                            reaction_log.borrow_mut().push("reaction");
                            Ok(value)
                        }),
                        None,
                    );
                    drop(reaction);
                    let _ = resolve.call((PromiseResolution::Value(1),));
                    Ok(())
                }),
                0.0,
            )
        })
        .expect("timer registration under the owner");
        let _ = p.await_result().await;
        log.borrow_mut().push("root");
    })
    .unwrap();
    let order = order.borrow();
    assert!(
        *order == ["reaction", "root"],
        "ran as {order:?}; the reaction runs in the callback's job checkpoint, before the root resumes"
    );
}
