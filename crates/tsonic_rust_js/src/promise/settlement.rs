use std::future::Future;
use std::rc::Rc;

use futures_util::FutureExt;
use tsonic_rust_runtime::Callable;

use super::{JsPromise, PromiseState};
use crate::JsValue;

pub enum PromiseResolution<T> {
    Value(T),
    Promise(JsPromise<T>),
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
    Callable<'static, (Input,), Result<JsPromise<Output>, JsValue>>;

impl<T: Clone + 'static> JsPromise<T> {
    pub fn create(executor: PromiseExecutor<'_, T>) -> Self {
        let promise = Self::pending();
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
        if let Err(reason) = executor.call((resolve, reject)) {
            promise.settle(Err(reason));
        }
        promise
    }

    fn resolve(&self, resolution: PromiseResolution<T>) {
        match resolution {
            PromiseResolution::Value(value) => self.settle(Ok(value)),
            PromiseResolution::Promise(source) => self.adopt(source),
        }
    }

    fn adopt(&self, source: JsPromise<T>) {
        if Rc::ptr_eq(&self.state, &source.state) {
            self.settle(Err(JsValue::from_error(crate::type_error(
                "Promise cannot resolve to itself",
            ))));
            return;
        }
        let claimed = match &mut *self.state.borrow_mut() {
            PromiseState::Pending { adopted, .. } if !*adopted => {
                crate::promise_jobs::require_dispatch_owner();
                *adopted = true;
                true
            }
            _ => false,
        };
        if !claimed {
            return;
        }
        let target = self.clone();
        if let Err(error) = crate::promise_jobs::enqueue(async move {
            target.complete_owned(source.into_result().await, true);
        }) {
            self.complete(Err(JsValue::from_error(error)), true);
        }
    }

    pub fn then<Output: Clone + 'static>(
        &self,
        fulfilled: Callable<'static, (T,), Result<Output, JsValue>>,
        rejected: Option<Callable<'static, (JsValue,), Result<Output, JsValue>>>,
    ) -> JsPromise<Output> {
        let source = self.clone();
        JsPromise::schedule(async move {
            match source.into_result().await {
                Ok(value) => fulfilled.call((value,)),
                Err(reason) => match rejected {
                    Some(callback) => callback.call((reason,)),
                    None => Err(reason),
                },
            }
        })
    }

    pub fn then_async<Output: Clone + 'static>(
        &self,
        fulfilled: PromiseThenCallback<T, Output>,
        rejected: Option<PromiseThenCallback<JsValue, Output>>,
    ) -> JsPromise<Output> {
        let source = self.clone();
        JsPromise::schedule(async move {
            let selected = match source.into_result().await {
                Ok(value) => fulfilled.call((value,)),
                Err(reason) => match rejected {
                    Some(callback) => callback.call((reason,)),
                    None => Err(reason),
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
                Err(reason) => rejected.call((reason,)),
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
        crate::promise_jobs::require_dispatch_owner();
        let promise = Self::pending();
        let completion = promise.clone();
        if let Err(error) = crate::promise_jobs::enqueue(
            future.map(move |result| completion.complete_owned(result, false)),
        ) {
            promise.settle(Err(JsValue::from_error(error)));
        }
        promise
    }
}
