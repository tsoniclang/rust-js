use std::cell::RefCell;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use tsonic_rust_runtime::{Callable, TsonicError, TsonicResult};

use crate::array::JsArray;
use crate::value::JsValue;

type PromiseFuture<'a, T> = Pin<Box<dyn Future<Output = TsonicResult<T>> + 'a>>;

enum PromiseState<'a, T> {
    Pending {
        future: Option<PromiseFuture<'a, T>>,
        waiters: Vec<Waker>,
    },
    Settled(TsonicResult<T>),
}

pub struct JsPromise<'a, T> {
    state: Rc<RefCell<PromiseState<'a, T>>>,
}

impl<'a, T> Clone for JsPromise<'a, T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<'a, T> JsPromise<'a, T> {
    pub fn from_infallible_factory<F, Fut>(factory: F) -> Self
    where
        F: FnOnce() -> Fut + 'a,
        Fut: Future<Output = T> + 'a,
    {
        Self::from_fallible_future(async move { Ok(factory().await) })
    }

    pub fn from_fallible_factory<F, Fut>(factory: F) -> Self
    where
        F: FnOnce() -> Fut + 'a,
        Fut: Future<Output = TsonicResult<T>> + 'a,
    {
        Self::from_fallible_future(factory())
    }

    pub fn resolved(value: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Ok(value)))),
        }
    }

    pub fn rejected(error: TsonicError) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Err(error)))),
        }
    }

    fn from_fallible_future(future: impl Future<Output = TsonicResult<T>> + 'a) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Pending {
                future: Some(Box::pin(future)),
                waiters: Vec::new(),
            })),
        }
    }
}

impl<'a, T: Clone + 'a> JsPromise<'a, T> {
    fn poll_result(&self, context: &mut Context<'_>) -> Poll<TsonicResult<T>> {
        let future = {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                PromiseState::Settled(result) => return Poll::Ready(result.clone()),
                PromiseState::Pending { future, waiters } => {
                    if !waiters
                        .iter()
                        .any(|waiter| waiter.will_wake(context.waker()))
                    {
                        waiters.push(context.waker().clone());
                    }
                    match future.take() {
                        Some(future) => future,
                        None => return Poll::Pending,
                    }
                }
            }
        };
        let mut future = future;
        match future.as_mut().poll(context) {
            Poll::Pending => {
                let mut state = self.state.borrow_mut();
                match &mut *state {
                    PromiseState::Pending {
                        future: pending_future,
                        ..
                    } => *pending_future = Some(future),
                    PromiseState::Settled(_) => {
                        panic!("Promise settled while its retained future was being polled")
                    }
                }
                Poll::Pending
            }
            Poll::Ready(result) => {
                let waiters = {
                    let mut state = self.state.borrow_mut();
                    let waiters = match &mut *state {
                        PromiseState::Pending { waiters, .. } => std::mem::take(waiters),
                        PromiseState::Settled(_) => {
                            panic!("Promise settled twice while polling its retained future")
                        }
                    };
                    *state = PromiseState::Settled(result.clone());
                    waiters
                };
                for waiter in waiters {
                    waiter.wake();
                }
                Poll::Ready(result)
            }
        }
    }

    pub async fn await_result(&self) -> TsonicResult<T> {
        poll_fn(|context| self.poll_result(context)).await
    }

    pub async fn await_value(&self) -> T {
        match self.await_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
    }

    pub fn finally_default(&self) -> Self {
        self.finally_callback(None)
    }

    pub fn finally(&self, callback: Callable<(), TsonicResult<()>>) -> Self {
        self.finally_callback(Some(callback))
    }

    fn finally_callback(&self, callback: Option<Callable<(), TsonicResult<()>>>) -> Self {
        let source = self.clone();
        Self::from_fallible_factory(move || async move {
            let result = source.await_result().await;
            if let Some(callback) = callback {
                callback.call(())?;
            }
            result
        })
    }
}

#[derive(Clone, Debug)]
pub struct PromiseFulfilledResult<T> {
    pub status: String,
    pub value: T,
}

#[derive(Clone, Debug)]
pub struct PromiseRejectedResult {
    pub status: String,
    pub reason: JsValue,
}

#[derive(Clone, Debug)]
pub enum PromiseSettledResult<T> {
    Fulfilled(PromiseFulfilledResult<T>),
    Rejected(PromiseRejectedResult),
}

impl<T> PromiseSettledResult<T> {
    pub fn status(&self) -> &String {
        match self {
            Self::Fulfilled(result) => &result.status,
            Self::Rejected(result) => &result.status,
        }
    }
}

pub fn promise_race<'a, T: Clone + 'a>(values: &JsArray<JsPromise<'a, T>>) -> JsPromise<'a, T> {
    let values = values.values();
    JsPromise::from_fallible_factory(move || async move {
        poll_fn(move |context| {
            for value in &values {
                let Some(value) = value else {
                    return Poll::Ready(Err(TsonicError::unsupported(
                        "Promise.race cannot consume an unproven sparse Promise array",
                    )));
                };
                if let Poll::Ready(result) = value.poll_result(context) {
                    return Poll::Ready(result);
                }
            }
            Poll::Pending
        })
        .await
    })
}

pub fn promise_any<'a, T: Clone + 'a>(values: &JsArray<JsPromise<'a, T>>) -> JsPromise<'a, T> {
    let values = values.values();
    JsPromise::from_fallible_factory(move || async move {
        if values.is_empty() {
            return Err(TsonicError::Js(crate::aggregate_error(
                "All promises were rejected",
            )));
        }
        let mut rejected = vec![false; values.len()];
        poll_fn(move |context| {
            for (index, value) in values.iter().enumerate() {
                let Some(value) = value else {
                    return Poll::Ready(Err(TsonicError::unsupported(
                        "Promise.any cannot consume an unproven sparse Promise array",
                    )));
                };
                if rejected[index] {
                    continue;
                }
                match value.poll_result(context) {
                    Poll::Ready(Ok(value)) => return Poll::Ready(Ok(value)),
                    Poll::Ready(Err(_)) => rejected[index] = true,
                    Poll::Pending => {}
                }
            }
            if rejected.iter().all(|rejected| *rejected) {
                Poll::Ready(Err(TsonicError::Js(crate::aggregate_error(
                    "All promises were rejected",
                ))))
            } else {
                Poll::Pending
            }
        })
        .await
    })
}

pub fn promise_all_settled<'a, T: Clone + 'a>(
    values: &JsArray<JsPromise<'a, T>>,
) -> JsPromise<'a, JsArray<PromiseSettledResult<T>>> {
    let values = values.values();
    JsPromise::from_infallible_factory(move || async move {
        let mut settled = vec![None; values.len()];
        poll_fn(move |context| {
            for (index, value) in values.iter().enumerate() {
                if settled[index].is_some() {
                    continue;
                }
                settled[index] = match value {
                    None => Some(PromiseSettledResult::Rejected(PromiseRejectedResult {
                        status: "rejected".to_string(),
                        reason: JsValue::String(crate::JsString::from_utf8(
                            "Promise.allSettled received a sparse Promise array",
                        )),
                    })),
                    Some(value) => match value.poll_result(context) {
                        Poll::Pending => None,
                        Poll::Ready(Ok(value)) => {
                            Some(PromiseSettledResult::Fulfilled(PromiseFulfilledResult {
                                status: "fulfilled".to_string(),
                                value,
                            }))
                        }
                        Poll::Ready(Err(error)) => {
                            Some(PromiseSettledResult::Rejected(PromiseRejectedResult {
                                status: "rejected".to_string(),
                                reason: JsValue::String(crate::JsString::from_utf8(
                                    &error.to_string(),
                                )),
                            }))
                        }
                    },
                };
            }
            if settled.iter().all(Option::is_some) {
                Poll::Ready(JsArray::from_dense(
                    settled.iter().map(|value| value.clone().unwrap()).collect(),
                ))
            } else {
                Poll::Pending
            }
        })
        .await
    })
}
