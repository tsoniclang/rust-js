use std::cell::Cell;
use std::future::{poll_fn, Future};
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::event_loop::block_on;
use tsonic_rust_js::promise::{JsPromise, PromiseReject, PromiseResolution, PromiseResolve};
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
fn a_factory_without_an_owner_rejects_with_the_exact_message() {
    let created = Rc::new(Cell::new(false));
    let observed = Rc::clone(&created);
    let promise: JsPromise<i32> = JsPromise::from_infallible_factory(move || {
        observed.set(true);
        async { 7 }
    });
    let result = tsonic_rust_runtime::block_on(promise.into_result());
    match result {
        Ok(value) => panic!("a factory without an owner resolved {value}"),
        Err(error) => assert!(error.to_string().contains(NO_LOOP), "{error}"),
    }
    assert!(!created.get(), "no Create body runs without an owner");
}
