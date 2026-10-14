use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use tsonic_rust_runtime::Callable;

use super::{JsPromise, PromiseState};
use crate::JsValue;

pub enum PromiseResolution<T> {
    Value(T),
    Promise(JsPromise<'static, T>),
}

impl<T: Clone> Clone for PromiseResolution<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.clone()),
            Self::Promise(promise) => Self::Promise(promise.clone()),
        }
    }
}

pub type PromiseResolve<T> = Callable<'static, (PromiseResolution<T>,), Result<(), JsValue>>;
pub type PromiseReject = Callable<'static, (JsValue,), Result<(), JsValue>>;
pub type PromiseExecutor<'environment, T> =
    Callable<'environment, (PromiseResolve<T>, PromiseReject), Result<(), JsValue>>;

type PromiseThenCallback<Input, Output> =
    Callable<'static, (Input,), Result<JsPromise<'static, Output>, JsValue>>;

impl<T: Clone + 'static> JsPromise<'static, T> {
    pub fn create(executor: PromiseExecutor<'_, T>) -> Self {
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

    fn resolve(&self, resolution: PromiseResolution<T>) {
        match resolution {
            PromiseResolution::Value(value) => self.settle(Ok(value)),
            PromiseResolution::Promise(promise) => {
                if Rc::ptr_eq(&self.state, &promise.state) {
                    self.settle(Err(JsValue::from_error(crate::type_error(
                        "Promise cannot resolve to itself",
                    ))));
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
        fulfilled: Callable<'static, (T,), Result<Output, JsValue>>,
        rejected: Option<Callable<'static, (JsValue,), Result<Output, JsValue>>>,
    ) -> JsPromise<'static, Output> {
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
        fulfilled: PromiseThenCallback<T, Output>,
        rejected: Option<PromiseThenCallback<JsValue, Output>>,
    ) -> JsPromise<'static, Output> {
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

    pub fn catch(&self, rejected: Callable<'static, (JsValue,), Result<T, JsValue>>) -> Self {
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

    pub fn finally(&self, callback: Callable<'static, (), Result<(), JsValue>>) -> Self {
        self.finally_callback(Some(callback))
    }

    fn finally_callback(
        &self,
        callback: Option<Callable<'static, (), Result<(), JsValue>>>,
    ) -> Self {
        let source = self.clone();
        Self::schedule(async move {
            let result = source.into_result().await;
            if let Some(callback) = callback {
                callback.call(())?;
            }
            result
        })
    }

    fn schedule(future: impl Future<Output = Result<T, JsValue>> + 'static) -> Self {
        let promise = Self::deferred();
        let completion = promise.clone();
        if let Err(error) = crate::promise_jobs::enqueue(async move {
            completion.settle(future.await);
        }) {
            promise.settle(Err(JsValue::from_error(error)));
        }
        promise
    }
}

impl<T> JsPromise<'_, T> {
    pub(crate) fn deferred() -> Self {
        Self {
            state: Rc::new(RefCell::new(PromiseState::Deferred {
                waiters: slab::Slab::new(),
            })),
        }
    }

    pub(crate) fn settle(&self, result: Result<T, JsValue>) {
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
}
