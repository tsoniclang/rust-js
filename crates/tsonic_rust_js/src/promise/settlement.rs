use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use tsonic_rust_runtime::{Callable, TsonicError};

use super::{JsPromise, PromiseState};

pub enum PromiseResolution<T, Error = TsonicError> {
    Value(T),
    Promise(JsPromise<'static, T, Error>),
}

impl<T: Clone, Error> Clone for PromiseResolution<T, Error> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.clone()),
            Self::Promise(promise) => Self::Promise(promise.clone()),
        }
    }
}

pub type PromiseResolve<T, Error = TsonicError> =
    Callable<(PromiseResolution<T, Error>,), Result<(), Error>>;
pub type PromiseReject<Error = TsonicError> = Callable<(Error,), Result<(), Error>>;
pub type PromiseExecutor<T, Error = TsonicError> =
    Callable<(PromiseResolve<T, Error>, PromiseReject<Error>), Result<(), Error>>;

impl<T: Clone + 'static, Error: Clone + From<tsonic_rust_runtime::JsError> + 'static>
    JsPromise<'static, T, Error>
{
    pub fn create(executor: PromiseExecutor<T, Error>) -> Self {
        let promise = Self::deferred();
        let resolve_target = promise.clone();
        let reject_target = promise.clone();
        let resolve = Callable::new(move |(value,)| {
            resolve_target.resolve(value);
            Ok(())
        });
        let reject = Callable::new(move |(reason,)| {
            reject_target.settle(Err(reason));
            Ok(())
        });
        if let Err(error) = executor.call((resolve, reject)) {
            promise.settle(Err(error));
        }
        promise
    }

    pub(crate) fn deferred() -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Deferred {
                waiters: slab::Slab::new(),
            })),
        }
    }

    pub(crate) fn settle(&self, result: Result<T, Error>) {
        let waiters = {
            let mut state = self.state.borrow_mut();
            let PromiseState::Deferred { waiters } = &mut *state else {
                return;
            };
            let waiters = std::mem::take(waiters);
            *state = PromiseState::Settled(result);
            waiters
        };
        for (_, waiter) in waiters {
            waiter.wake();
        }
    }

    fn resolve(&self, resolution: PromiseResolution<T, Error>) {
        match resolution {
            PromiseResolution::Value(value) => self.settle(Ok(value)),
            PromiseResolution::Promise(promise) => {
                if Rc::ptr_eq(&self.state, &promise.state) {
                    self.settle(Err(
                        crate::type_error("Promise cannot resolve to itself").into()
                    ));
                    return;
                }
                let waiters = {
                    let mut state = self.state.borrow_mut();
                    let PromiseState::Deferred { waiters } = &mut *state else {
                        return;
                    };
                    let waiters = std::mem::take(waiters);
                    *state = PromiseState::Pending {
                        future: Some(Box::pin(promise.into_result())),
                    };
                    waiters
                };
                for (_, waiter) in waiters {
                    waiter.wake();
                }
            }
        }
    }

    pub fn then<Output: Clone + 'static>(
        &self,
        fulfilled: Callable<(T,), Result<Output, Error>>,
        rejected: Option<Callable<(Error,), Result<Output, Error>>>,
    ) -> JsPromise<'static, Output, Error> {
        let source = self.clone();
        JsPromise::schedule(async move {
            match source.into_result().await {
                Ok(value) => fulfilled.call((value,)),
                Err(error) => match rejected {
                    Some(callback) => callback.call((error,)),
                    None => Err(error),
                },
            }
        })
    }

    pub fn then_async<Output: Clone + 'static>(
        &self,
        fulfilled: Callable<(T,), Result<JsPromise<'static, Output, Error>, Error>>,
        rejected: Option<Callable<(Error,), Result<JsPromise<'static, Output, Error>, Error>>>,
    ) -> JsPromise<'static, Output, Error> {
        let source = self.clone();
        JsPromise::schedule(async move {
            let selected = match source.into_result().await {
                Ok(value) => fulfilled.call((value,)),
                Err(error) => match rejected {
                    Some(callback) => callback.call((error,)),
                    None => Err(error),
                },
            }?;
            selected.into_result().await
        })
    }

    pub fn catch(&self, rejected: Callable<(Error,), Result<T, Error>>) -> Self {
        let source = self.clone();
        Self::schedule(async move {
            match source.into_result().await {
                Ok(value) => Ok(value),
                Err(error) => rejected.call((error,)),
            }
        })
    }

    pub fn finally_default(&self) -> Self {
        self.finally_callback(None)
    }

    pub fn finally(&self, callback: Callable<(), Result<(), Error>>) -> Self {
        self.finally_callback(Some(callback))
    }

    fn finally_callback(&self, callback: Option<Callable<(), Result<(), Error>>>) -> Self {
        let source = self.clone();
        Self::schedule(async move {
            let result = source.into_result().await;
            if let Some(callback) = callback {
                callback.call(())?;
            }
            result
        })
    }

    fn schedule(future: impl Future<Output = Result<T, Error>> + 'static) -> Self {
        let promise = Self::deferred();
        let completion = promise.clone();
        if let Err(error) = crate::promise_jobs::enqueue(async move {
            completion.settle(future.await);
        }) {
            promise.settle(Err(error.into()));
        }
        promise
    }
}
