use std::cell::RefCell;
use std::future::Future;
use std::io::Write;
use std::rc::Rc;

use futures_util::FutureExt;
use tsonic_rust_runtime::ErrorObject;

use crate::JsValue;

mod awaiting;
mod combinators;
mod settlement;
mod waiters;

pub use combinators::{promise_all, promise_all_settled, promise_any, promise_race};
pub use settlement::{PromiseExecutor, PromiseReject, PromiseResolution, PromiseResolve};

use waiters::Waiters;

enum PromiseState<T> {
    Pending {
        waiters: Waiters,
        adopted: bool,
        observed: bool,
    },
    Fulfilled(T),
    Rejected {
        reason: JsValue,
        reported: bool,
    },
}

impl<T> PromiseState<T> {
    fn pending() -> Self {
        Self::Pending {
            waiters: Waiters::default(),
            adopted: false,
            observed: false,
        }
    }

    fn into_settled(self) -> Result<T, JsValue> {
        match self {
            Self::Fulfilled(value) => Ok(value),
            Self::Rejected { reason, .. } => Err(reason),
            Self::Pending { .. } => unreachable!("a settled Promise is read only after settlement"),
        }
    }
}

impl<T: Clone> PromiseState<T> {
    fn clone_settled(&self) -> Result<T, JsValue> {
        match self {
            Self::Fulfilled(value) => Ok(value.clone()),
            Self::Rejected { reason, .. } => Err(reason.clone()),
            Self::Pending { .. } => unreachable!("a settled Promise is read only after settlement"),
        }
    }
}

pub struct JsPromise<T> {
    state: Rc<RefCell<PromiseState<T>>>,
}

impl<T> Clone for JsPromise<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<T> JsPromise<T> {
    pub fn resolved(value: T) -> Self {
        Self::with_state(PromiseState::Fulfilled(value))
    }

    pub fn rejected(reason: JsValue) -> Self {
        Self::with_state(PromiseState::Rejected {
            reason,
            reported: false,
        })
    }

    pub fn from_result(result: Result<T, JsValue>) -> Self {
        match result {
            Ok(value) => Self::resolved(value),
            Err(reason) => Self::rejected(reason),
        }
    }

    pub fn observe_discarded(self) {
        let report = {
            let mut state = self.state.borrow_mut();
            match &mut *state {
                PromiseState::Fulfilled(_) | PromiseState::Rejected { reported: true, .. } => None,
                PromiseState::Rejected { reason, reported } => {
                    *reported = true;
                    Some(reason.clone())
                }
                PromiseState::Pending { observed, .. } => {
                    *observed = true;
                    None
                }
            }
        };
        if let Some(reason) = report {
            report_discarded_rejection(reason);
        }
    }

    fn with_state(state: PromiseState<T>) -> Self {
        Self {
            state: Rc::new(RefCell::new(state)),
        }
    }

    pub(crate) fn pending() -> Self {
        Self::with_state(PromiseState::pending())
    }

    pub(crate) fn settle(&self, result: Result<T, JsValue>) {
        self.complete(result, false);
    }

    fn complete(&self, result: Result<T, JsValue>, adoption: bool) {
        let (waiters, report) = {
            let mut state = self.state.borrow_mut();
            let PromiseState::Pending {
                waiters,
                adopted,
                observed,
            } = &mut *state
            else {
                return;
            };
            if *adopted && !adoption {
                return;
            }
            let waiters = std::mem::take(waiters);
            let observed = *observed;
            let report = match &result {
                Err(reason) if observed => Some(reason.clone()),
                _ => None,
            };
            *state = match result {
                Ok(value) => PromiseState::Fulfilled(value),
                Err(reason) => PromiseState::Rejected {
                    reason,
                    reported: observed,
                },
            };
            (waiters, report)
        };
        waiters.wake_all();
        if let Some(reason) = report {
            report_discarded_rejection(reason);
        }
    }
}

impl<T: 'static> JsPromise<T> {
    pub fn from_infallible_factory<Create, Body>(create: Create) -> Self
    where
        Create: FnOnce() -> Body,
        Body: Future<Output = T> + 'static,
    {
        Self::admit_body(move || create().map(Ok::<T, JsValue>))
    }

    pub fn from_fallible_factory<Create, Body>(create: Create) -> Self
    where
        Create: FnOnce() -> Body,
        Body: Future<Output = Result<T, JsValue>> + 'static,
    {
        Self::admit_body(create)
    }

    fn admit_body<Body>(create: impl FnOnce() -> Body) -> Self
    where
        Body: Future<Output = Result<T, JsValue>> + 'static,
    {
        if let Err(error) = crate::promise_jobs::ensure_running() {
            return Self::rejected(JsValue::from_error(error));
        }
        let body = create();
        let promise = Self::pending();
        let completion = promise.clone();
        if let Err(error) =
            crate::promise_jobs::start(body.map(move |result| completion.settle(result)))
        {
            promise.settle(Err(JsValue::from_error(error)));
        }
        promise
    }
}

impl<T: Clone> JsPromise<T> {
    pub async fn into_result(self) -> Result<T, JsValue> {
        awaiting::PromiseWait::new(&self.state).await;
        match Rc::try_unwrap(self.state) {
            Ok(state) => state.into_inner().into_settled(),
            Err(state) => state.borrow().clone_settled(),
        }
    }

    pub async fn into_value(self) -> T {
        match self.into_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
    }

    pub async fn await_result(&self) -> Result<T, JsValue> {
        awaiting::PromiseWait::new(&self.state).await;
        self.state.borrow().clone_settled()
    }

    pub async fn await_value(&self) -> T {
        match self.await_result().await {
            Ok(value) => value,
            Err(error) => panic!("compiler-proven infallible Promise rejected: {error}"),
        }
    }
}

fn report_discarded_rejection(reason: JsValue) {
    if crate::promise_jobs::can_admit().is_ok() {
        crate::promise_jobs::enqueue(async move {
            write_discarded_rejection(&reason);
            crate::promise_jobs::request_exit(1);
        })
        .expect("report admission was checked on the owning thread");
        return;
    }
    write_discarded_rejection(&reason);
    if !crate::promise_jobs::request_exit(1) {
        std::process::exit(1);
    }
}

fn write_discarded_rejection(reason: &JsValue) {
    #[cfg(test)]
    REPORTS.with(|reports| {
        let _ = write_report(&mut *reports.borrow_mut(), reason);
    });
    let _ = write_report(&mut std::io::stderr().lock(), reason);
}

fn write_report(output: &mut impl Write, reason: &JsValue) -> std::io::Result<()> {
    match reason.as_error() {
        Some(error) => writeln!(output, "{}", error.error_message()),
        None => match reason {
            JsValue::Null => writeln!(output, "null"),
            JsValue::String(text) => writeln!(output, "{text}"),
            JsValue::Utf16String(text) => match text.to_utf8() {
                Ok(text) => writeln!(output, "{text}"),
                Err(_) => writeln!(output, "{reason}"),
            },
            other => writeln!(output, "{other}"),
        },
    }
}

#[cfg(test)]
thread_local! {
    static REPORTS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_reports() -> String {
    REPORTS.with(|reports| String::from_utf8(reports.take()).expect("reports are UTF-8 text"))
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
