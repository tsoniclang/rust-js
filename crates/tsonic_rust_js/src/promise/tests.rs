use super::*;
use crate::equality::JsStrictEqual;
use crate::event_loop::block_on;
use crate::{JsArray, JsValue};
use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use tsonic_rust_runtime::{
    Callable, ErrorObject, JsError, JsErrorKind, MutableJsError, WritableErrorObject,
};

#[test]
fn settled_payloads_keep_native_borrows_and_consume_without_copying() {
    let text = String::from("borrowed payload");
    let promise: JsPromise<'_, &str> = JsPromise::resolved(text.as_str());
    let mut result = std::pin::pin!(promise.into_result());
    assert_eq!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Ok(text.as_str()))
    );
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
    fn exported_failure(reason: JsValue) -> JsPromise<'static, i32> {
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
    let values = JsArray::<JsPromise<'_, i32>>::new();
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
    let values = JsArray::<JsPromise<'_, i32>>::new();
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
            JsArray::<JsPromise<'_, i32>>::new(),
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
        let first = JsPromise::deferred();
        let second = JsPromise::deferred();
        let values = JsArray::from_dense(vec![first.clone(), second.clone()]);
        let all = promise_all(&values);
        let settled = promise_all_settled(&values);
        crate::promise_jobs::enqueue(async move {
            second.settle(Ok(29));
            crate::promise_jobs::enqueue(async move { first.settle(Ok(17)); }).unwrap();
        }).unwrap();
        assert_eq!(all.into_result().await.unwrap().values(), [17, 29]);
        let settled = settled.into_result().await.unwrap();
        assert!(matches!(settled.get(0), Some(PromiseSettledResult::Fulfilled(value)) if value.value == 17));
        assert!(matches!(settled.get(1), Some(PromiseSettledResult::Fulfilled(value)) if value.value == 29));
    }).unwrap();
}

#[test]
fn all_fails_fast_without_waiting_for_an_unsettled_input() {
    block_on(async {
        let reason = JsValue::from_error(JsError::error("early failure"));
        let values = JsArray::from_dense(vec![
            JsPromise::<i32>::deferred(),
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
        let first = JsPromise::<i32>::deferred();
        let second = JsPromise::deferred();
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
        let first = JsPromise::<i32>::deferred();
        let second = JsPromise::deferred();
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
