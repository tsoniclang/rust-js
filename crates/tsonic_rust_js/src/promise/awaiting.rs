use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::waiters::WaiterKey;
use super::PromiseState;

pub(super) struct PromiseWait<'promise, T> {
    state: &'promise RefCell<PromiseState<T>>,
    waiter: Option<WaiterKey>,
}

impl<'promise, T> PromiseWait<'promise, T> {
    pub(super) fn new(state: &'promise RefCell<PromiseState<T>>) -> Self {
        Self {
            state,
            waiter: None,
        }
    }
}

impl<T> Future for PromiseWait<'_, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let waiting = self.get_mut();
        let mut state = waiting.state.borrow_mut();
        match &mut *state {
            PromiseState::Pending { waiters, .. } => {
                match waiting.waiter {
                    Some(key) => waiters.update(key, context.waker()),
                    None => waiting.waiter = Some(waiters.register(context.waker())),
                }
                Poll::Pending
            }
            PromiseState::Fulfilled(_) | PromiseState::Rejected { .. } => {
                waiting.waiter = None;
                Poll::Ready(())
            }
        }
    }
}

impl<T> Drop for PromiseWait<'_, T> {
    fn drop(&mut self) {
        if let Some(key) = self.waiter {
            if let PromiseState::Pending { waiters, .. } = &mut *self.state.borrow_mut() {
                waiters.remove(key);
            }
        }
    }
}
