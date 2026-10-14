use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::future::Shared;
use futures_util::FutureExt;

use super::{JsPromise, PromiseFuture, PromiseState};
use crate::JsValue;

pub(super) struct PromiseAwait<'promise, 'future, T> {
    promise: &'promise JsPromise<'future, T>,
    deferred_waiter: Option<usize>,
    shared: Option<Shared<PromiseFuture<'future, T>>>,
}

impl<'promise, 'future, T> PromiseAwait<'promise, 'future, T> {
    pub(super) fn new(promise: &'promise JsPromise<'future, T>) -> Self {
        Self {
            promise,
            deferred_waiter: None,
            shared: None,
        }
    }
}

impl<T: Clone> Future for PromiseAwait<'_, '_, T> {
    type Output = Result<T, JsValue>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let awaiting = self.get_mut();
        if awaiting.shared.is_none() {
            let mut state = awaiting.promise.state.borrow_mut();
            match &mut *state {
                PromiseState::Settled(result) => return Poll::Ready(result.clone()),
                PromiseState::Deferred { waiters } => {
                    if let Some(key) = awaiting.deferred_waiter {
                        waiters[key].clone_from(context.waker());
                    } else {
                        awaiting.deferred_waiter = Some(waiters.insert(context.waker().clone()));
                    }
                    return Poll::Pending;
                }
                PromiseState::Pending { future } => {
                    let shared = future
                        .take()
                        .expect("pending Promise future owner")
                        .shared();
                    awaiting.shared = Some(shared.clone());
                    *state = PromiseState::Shared(shared);
                }
                PromiseState::Shared(shared) => awaiting.shared = Some(shared.clone()),
            }
        }
        Pin::new(
            awaiting
                .shared
                .as_mut()
                .expect("selected shared Promise future"),
        )
        .poll(context)
    }
}

impl<T> Drop for PromiseAwait<'_, '_, T> {
    fn drop(&mut self) {
        if let Some(key) = self.deferred_waiter {
            if let PromiseState::Deferred { waiters } = &mut *self.promise.state.borrow_mut() {
                waiters.remove(key);
            }
        }
    }
}
