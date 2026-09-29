use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Waker;

use futures_util::future::Shared;
use slab::Slab;

use tsonic_rust_runtime::TsonicError;

mod awaiting;
mod combinators;
mod settlement;

pub use combinators::{promise_all_settled, promise_any, promise_race};
pub use settlement::{PromiseExecutor, PromiseReject, PromiseResolution, PromiseResolve};

type PromiseFuture<'a, T, Error> = Pin<Box<dyn Future<Output = Result<T, Error>> + 'a>>;

enum PromiseState<'a, T, Error> {
    Deferred {
        waiters: Slab<Waker>,
    },
    Pending {
        future: Option<PromiseFuture<'a, T, Error>>,
    },
    Shared(Shared<PromiseFuture<'a, T, Error>>),
    Settled(Result<T, Error>),
}

pub struct JsPromise<'a, T, Error = TsonicError> {
    state: Rc<RefCell<PromiseState<'a, T, Error>>>,
}

impl<'a, T, Error> Clone for JsPromise<'a, T, Error> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<'a, T, Error: 'a> JsPromise<'a, T, Error> {
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
        Fut: Future<Output = Result<T, Error>> + 'a,
    {
        Self::from_fallible_future(factory())
    }

    pub fn resolved(value: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Ok(value)))),
        }
    }

    pub fn rejected(error: Error) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Settled(Err(error)))),
        }
    }

    fn from_fallible_future(future: impl Future<Output = Result<T, Error>> + 'a) -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Pending {
                future: Some(Box::pin(future)),
            })),
        }
    }
}

impl<'a, T: Clone + 'a, Error: Clone + 'a> JsPromise<'a, T, Error> {
    pub async fn into_result(self) -> Result<T, Error> {
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

    pub async fn into_value(self) -> T
    where
        Error: std::fmt::Display,
    {
        match self.into_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
    }

    pub async fn await_result(&self) -> Result<T, Error> {
        awaiting::PromiseAwait::new(self).await
    }

    pub async fn await_value(&self) -> T
    where
        Error: std::fmt::Display,
    {
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
pub struct PromiseRejectedResult<Error = TsonicError> {
    pub status: String,
    pub reason: Error,
}

#[derive(Clone, Debug)]
pub enum PromiseSettledResult<T, Error = TsonicError> {
    Fulfilled(PromiseFulfilledResult<T>),
    Rejected(PromiseRejectedResult<Error>),
}

impl<T, Error> PromiseSettledResult<T, Error> {
    pub fn status(&self) -> &String {
        match self {
            Self::Fulfilled(result) => &result.status,
            Self::Rejected(result) => &result.status,
        }
    }
}
