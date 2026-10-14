use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Waker;

use futures_util::future::Shared;
use slab::Slab;

use crate::JsValue;

mod awaiting;
mod combinators;
mod settlement;

pub use combinators::{promise_all, promise_all_settled, promise_any, promise_race};
pub use settlement::{PromiseExecutor, PromiseReject, PromiseResolution, PromiseResolve};

type PromiseFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, JsValue>> + 'a>>;

enum PromiseState<'a, T> {
    Deferred {
        waiters: Slab<Waker>,
    },
    Pending {
        future: Option<PromiseFuture<'a, T>>,
    },
    Shared(Shared<PromiseFuture<'a, T>>),
    Settled(Result<T, JsValue>),
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
        Fut: Future<Output = Result<T, JsValue>> + 'a,
    {
        Self::from_fallible_future(factory())
    }

    pub fn resolved(value: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Ok(value)))),
        }
    }

    pub fn rejected(error: JsValue) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Err(error)))),
        }
    }

    fn from_fallible_future(future: impl Future<Output = Result<T, JsValue>> + 'a) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Pending {
                future: Some(Box::pin(future)),
            })),
        }
    }
}

impl<'a, T: Clone + 'a> JsPromise<'a, T> {
    pub async fn into_result(self) -> Result<T, JsValue> {
        match Rc::try_unwrap(self.state) {
            Ok(state) => match state.into_inner() {
                PromiseState::Settled(result) => result,
                PromiseState::Deferred { .. } => std::future::pending().await,
                PromiseState::Pending { future, .. } => {
                    future
                        .expect("an exclusively owned Promise cannot be polling")
                        .await
                }
                PromiseState::Shared(future) => future.await,
            },
            Err(state) => Self { state }.await_result().await,
        }
    }

    pub async fn into_value(self) -> T {
        match self.into_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
    }

    pub async fn await_result(&self) -> Result<T, JsValue> {
        awaiting::PromiseAwait::new(self).await
    }

    pub async fn await_value(&self) -> T {
        match self.await_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
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

#[cfg(test)]
mod tests;
