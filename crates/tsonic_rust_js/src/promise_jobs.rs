use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::stream::{FuturesUnordered, Stream};
use tsonic_rust_runtime::JsError;

const MAX_PENDING_JOBS: usize = 1 << 20;
const MAX_COMPLETIONS_PER_TURN: usize = 1024;

type Job = Pin<Box<dyn Future<Output = ()>>>;

struct Jobs {
    active: Option<FuturesUnordered<Job>>,
    incoming: Vec<Job>,
    count: usize,
}

pub(crate) fn has_jobs() -> bool {
    JOBS.with_borrow(|jobs| jobs.count != 0)
}

thread_local! {
    static JOBS: RefCell<Jobs> = RefCell::new(Jobs {
        active: Some(FuturesUnordered::new()),
        incoming: Vec::new(),
        count: 0,
    });
}

pub(crate) fn enqueue(job: impl Future<Output = ()> + 'static) -> Result<(), JsError> {
    JOBS.with_borrow_mut(|jobs| {
        if jobs.count == MAX_PENDING_JOBS {
            return Err(crate::range_error(
                "pending Promise jobs exceed the finite queue limit",
            ));
        }
        jobs.count += 1;
        let job: Job = Box::pin(job);
        match &jobs.active {
            Some(active) => active.push(job),
            None => jobs.incoming.push(job),
        }
        Ok(())
    })
}

struct ActiveJobs(Option<FuturesUnordered<Job>>);

impl Drop for ActiveJobs {
    fn drop(&mut self) {
        JOBS.with_borrow_mut(|jobs| {
            let active = self.0.take().expect("active Promise job owner");
            for job in jobs.incoming.drain(..) {
                active.push(job);
            }
            jobs.count = active.len();
            jobs.active = Some(active);
        });
    }
}

pub(crate) fn poll(context: &mut Context<'_>) -> bool {
    let mut active = ActiveJobs(Some(JOBS.with_borrow_mut(|jobs| {
        jobs.active
            .take()
            .expect("Promise job polling cannot reenter")
    })));
    let queue = active.0.as_mut().expect("active Promise job owner");
    let mut completed = 0;
    while completed < MAX_COMPLETIONS_PER_TURN {
        match Pin::new(&mut *queue).poll_next(context) {
            Poll::Ready(Some(())) => {
                completed += 1;
                JOBS.with_borrow_mut(|jobs| jobs.count -= 1);
            }
            Poll::Ready(None) | Poll::Pending => break,
        }
    }
    if completed == MAX_COMPLETIONS_PER_TURN || JOBS.with_borrow(|jobs| !jobs.incoming.is_empty()) {
        context.waker().wake_by_ref();
    }
    completed > 0
}

#[cfg(test)]
mod tests {
    use super::{enqueue, JOBS, MAX_PENDING_JOBS};
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn a_full_queue_rejects_before_polling_or_retaining_the_job() {
        JOBS.with_borrow_mut(|jobs| {
            assert_eq!(jobs.count, 0);
            jobs.count = MAX_PENDING_JOBS;
        });
        let ran = Rc::new(Cell::new(false));
        let captured = Rc::clone(&ran);
        let rejected = enqueue(async move { captured.set(true) });
        JOBS.with_borrow_mut(|jobs| jobs.count = 0);
        assert!(rejected.is_err());
        assert!(!ran.get());
        assert_eq!(Rc::strong_count(&ran), 1);
    }

    #[test]
    fn rejected_admission_settles_the_returned_promise_and_releases_captures() {
        use crate::promise::JsPromise;
        use tsonic_rust_runtime::{Callable, JsErrorKind};

        let ran = Rc::new(Cell::new(false));
        let captured = Rc::clone(&ran);
        JOBS.with_borrow_mut(|jobs| {
            assert_eq!(jobs.count, 0);
            jobs.count = MAX_PENDING_JOBS;
        });
        let result = JsPromise::<i32>::resolved(1).then(
            Callable::new(move |(value,)| {
                captured.set(true);
                Ok(value)
            }),
            None,
        );
        JOBS.with_borrow_mut(|jobs| jobs.count = 0);
        let error = crate::event_loop::block_on(result.into_result())
            .unwrap()
            .unwrap_err();
        assert_eq!(error.source_error().kind(), JsErrorKind::RangeError);
        assert!(!ran.get());
        assert_eq!(Rc::strong_count(&ran), 1);
    }
}
