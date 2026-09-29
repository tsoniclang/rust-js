use futures_util::stream::{FuturesUnordered, StreamExt};

use super::{JsPromise, PromiseFulfilledResult, PromiseRejectedResult, PromiseSettledResult};
use crate::array::JsArray;

pub fn promise_race<'a, T: Clone + 'a, Error: Clone + 'a>(
    values: &JsArray<JsPromise<'a, T, Error>>,
) -> JsPromise<'a, T, Error> {
    let values = values.values();
    JsPromise::from_fallible_factory(move || async move {
        let mut pending: FuturesUnordered<_> =
            values.into_iter().map(JsPromise::into_result).collect();
        match pending.next().await {
            Some(result) => result,
            None => std::future::pending().await,
        }
    })
}

pub fn promise_any<'a, T: Clone + 'a, Error: Clone + From<tsonic_rust_runtime::JsError> + 'a>(
    values: &JsArray<JsPromise<'a, T, Error>>,
) -> JsPromise<'a, T, Error> {
    let values = values.values();
    JsPromise::from_fallible_factory(move || async move {
        let mut pending: FuturesUnordered<_> =
            values.into_iter().map(JsPromise::into_result).collect();
        while let Some(result) = pending.next().await {
            if let Ok(value) = result {
                return Ok(value);
            }
        }
        Err(crate::aggregate_error("All promises were rejected").into())
    })
}

pub fn promise_all_settled<'a, T: Clone + 'a, Error: Clone + 'a>(
    values: &JsArray<JsPromise<'a, T, Error>>,
) -> JsPromise<'a, JsArray<PromiseSettledResult<T, Error>>, Error> {
    let values = values.values();
    JsPromise::from_infallible_factory(move || async move {
        let mut settled = vec![None; values.len()];
        let mut pending: FuturesUnordered<_> = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| async move { (index, value.into_result().await) })
            .collect();
        while let Some((index, result)) = pending.next().await {
            settled[index] = Some(match result {
                Ok(value) => PromiseSettledResult::Fulfilled(PromiseFulfilledResult {
                    status: "fulfilled".to_string(),
                    value,
                }),
                Err(reason) => PromiseSettledResult::Rejected(PromiseRejectedResult {
                    status: "rejected".to_string(),
                    reason,
                }),
            });
        }
        JsArray::from_dense(
            settled
                .into_iter()
                .map(|value| value.expect("completed Promise settlement"))
                .collect(),
        )
    })
}
