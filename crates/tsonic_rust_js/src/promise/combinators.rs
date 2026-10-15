use futures_util::future::{join_all, try_join_all};
use futures_util::stream::{FuturesUnordered, StreamExt};

use super::{JsPromise, PromiseFulfilledResult, PromiseRejectedResult, PromiseSettledResult};
use crate::array::JsArray;
use crate::JsValue;

pub fn promise_all<T: Clone + 'static>(values: &JsArray<JsPromise<T>>) -> JsPromise<JsArray<T>> {
    let values = values.values();
    if values.is_empty() {
        return JsPromise::resolved(JsArray::from_dense(Vec::new()));
    }
    JsPromise::from_fallible_factory(move || async move {
        try_join_all(values.into_iter().map(JsPromise::into_result))
            .await
            .map(JsArray::from_dense)
    })
}

pub fn promise_race<T: Clone + 'static>(values: &JsArray<JsPromise<T>>) -> JsPromise<T> {
    let values = values.values();
    if values.is_empty() {
        return JsPromise::pending();
    }
    JsPromise::from_fallible_factory(move || async move {
        let mut pending: FuturesUnordered<_> =
            values.into_iter().map(JsPromise::into_result).collect();
        match pending.next().await {
            Some(result) => result,
            None => std::future::pending().await,
        }
    })
}

pub fn promise_any<T: Clone + 'static>(values: &JsArray<JsPromise<T>>) -> JsPromise<T> {
    let values = values.values();
    if values.is_empty() {
        return JsPromise::rejected(JsValue::from_error(crate::aggregate_error(
            "All promises were rejected",
        )));
    }
    JsPromise::from_fallible_factory(move || async move {
        let mut pending: FuturesUnordered<_> =
            values.into_iter().map(JsPromise::into_result).collect();
        while let Some(result) = pending.next().await {
            if let Ok(value) = result {
                return Ok(value);
            }
        }
        Err(JsValue::from_error(crate::aggregate_error(
            "All promises were rejected",
        )))
    })
}

pub fn promise_all_settled<T: Clone + 'static>(
    values: &JsArray<JsPromise<T>>,
) -> JsPromise<JsArray<PromiseSettledResult<T>>> {
    let values = values.values();
    if values.is_empty() {
        return JsPromise::resolved(JsArray::from_dense(Vec::new()));
    }
    JsPromise::from_infallible_factory(move || async move {
        let settled = join_all(values.into_iter().map(|value| async move {
            match value.into_result().await {
                Ok(value) => PromiseSettledResult::Fulfilled(PromiseFulfilledResult {
                    status: "fulfilled".to_string(),
                    value,
                }),
                Err(reason) => PromiseSettledResult::Rejected(PromiseRejectedResult {
                    status: "rejected".to_string(),
                    reason,
                }),
            }
        }))
        .await;
        JsArray::from_dense(settled)
    })
}
