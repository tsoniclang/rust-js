use std::cell::{OnceCell, RefCell};
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
    JOBS.with(|owner| owner.get().is_some_and(|jobs| jobs.borrow().count != 0))
}

impl Default for Jobs {
    fn default() -> Self {
        Self {
            active: Some(FuturesUnordered::new()),
            incoming: Vec::new(),
            count: 0,
        }
    }
}

thread_local! {
    static JOBS: OnceCell<RefCell<Jobs>> = const { OnceCell::new() };
}

pub(crate) fn enqueue(job: impl Future<Output = ()> + 'static) -> Result<(), JsError> {
    JOBS.with(|owner| {
        let mut jobs = owner.get_or_init(RefCell::default).borrow_mut();
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
        JOBS.with(|owner| {
            let mut jobs = owner.get().expect("active Promise job owner").borrow_mut();
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
    JOBS.with(|owner| {
        let Some(jobs) = owner.get() else {
            return false;
        };
        let mut active = ActiveJobs(Some(
            jobs.borrow_mut()
                .active
                .take()
                .expect("Promise job polling cannot reenter"),
        ));
        let queue = active.0.as_mut().expect("active Promise job owner");
        let mut completed = 0;
        while completed < MAX_COMPLETIONS_PER_TURN {
            match Pin::new(&mut *queue).poll_next(context) {
                Poll::Ready(Some(())) => {
                    completed += 1;
                    jobs.borrow_mut().count -= 1;
                }
                Poll::Ready(None) | Poll::Pending => break,
            }
        }
        if completed == MAX_COMPLETIONS_PER_TURN || !jobs.borrow().incoming.is_empty() {
            context.waker().wake_by_ref();
        }
        completed > 0
    })
}

#[cfg(test)]
mod tests {
    use super::{enqueue, has_jobs, poll, JOBS, MAX_PENDING_JOBS};
    use std::cell::Cell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;
    use std::task::{Context, Waker};

    #[test]
    fn a_full_queue_rejects_before_polling_or_retaining_the_job() {
        JOBS.with(|owner| {
            let mut jobs = owner.get_or_init(std::cell::RefCell::default).borrow_mut();
            assert_eq!(jobs.count, 0);
            jobs.count = MAX_PENDING_JOBS;
        });
        let ran = Rc::new(Cell::new(false));
        let captured = Rc::clone(&ran);
        let rejected = enqueue(async move { captured.set(true) });
        JOBS.with(|owner| owner.get().unwrap().borrow_mut().count = 0);
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
        JOBS.with(|owner| {
            let mut jobs = owner.get_or_init(std::cell::RefCell::default).borrow_mut();
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
        JOBS.with(|owner| owner.get().unwrap().borrow_mut().count = 0);
        let error = crate::event_loop::block_on(result.into_result())
            .unwrap()
            .unwrap_err();
        assert_eq!(error.source_error().kind(), JsErrorKind::RangeError);
        assert!(!ran.get());
        assert_eq!(Rc::strong_count(&ran), 1);
    }

    #[test]
    fn cold_queries_and_polling_do_not_initialize_the_job_queue() {
        JOBS.with(|owner| assert!(owner.get().is_none()));
        assert!(!has_jobs());
        assert!(!poll(&mut Context::from_waker(Waker::noop())));
        JOBS.with(|owner| assert!(owner.get().is_none()));
    }

    #[test]
    fn admission_initializes_one_owner_and_queries_retain_its_identity() {
        enqueue(async {}).unwrap();
        let first = JOBS.with(|owner| std::ptr::from_ref(owner.get().unwrap()));
        assert!(has_jobs());
        enqueue(async {}).unwrap();
        assert!(poll(&mut Context::from_waker(Waker::noop())));
        assert!(!has_jobs());
        JOBS.with(|owner| {
            let current = owner.get().unwrap();
            assert_eq!(first, std::ptr::from_ref(current));
            assert_eq!(current.borrow().count, 0);
        });
    }

    #[test]
    fn reentrant_admission_keeps_exact_order_and_releases_captures() {
        let order = Rc::new(std::cell::RefCell::new(Vec::new()));
        let outer = Rc::clone(&order);
        enqueue(async move {
            outer.borrow_mut().push(1);
            let inner = Rc::clone(&outer);
            enqueue(async move { inner.borrow_mut().push(3) }).unwrap();
            outer.borrow_mut().push(2);
        })
        .unwrap();
        let mut context = Context::from_waker(Waker::noop());
        assert!(poll(&mut context));
        assert_eq!(*order.borrow(), [1, 2]);
        assert!(has_jobs());
        assert_eq!(Rc::strong_count(&order), 2);
        assert!(poll(&mut context));
        assert_eq!(*order.borrow(), [1, 2, 3]);
        assert!(!has_jobs());
        assert_eq!(Rc::strong_count(&order), 1);
    }

    #[test]
    fn panicking_jobs_restore_the_live_queue_and_preserve_uninvoked_work() {
        let ran = Rc::new(Cell::new(false));
        let captured = Rc::clone(&ran);
        enqueue(async { panic!("original Promise job failure") }).unwrap();
        enqueue(async move { captured.set(true) }).unwrap();
        let mut context = Context::from_waker(Waker::noop());
        let failure = catch_unwind(AssertUnwindSafe(|| poll(&mut context)));
        assert!(failure.is_err());
        assert!(!ran.get());
        assert!(has_jobs());
        assert_eq!(Rc::strong_count(&ran), 2);
        assert!(poll(&mut context));
        assert!(ran.get());
        assert!(!has_jobs());
        assert_eq!(Rc::strong_count(&ran), 1);
    }

    #[test]
    fn recursive_polling_is_rejected_without_losing_the_outer_queue() {
        let rejected = Rc::new(Cell::new(false));
        let captured = Rc::clone(&rejected);
        enqueue(async move {
            let mut context = Context::from_waker(Waker::noop());
            captured.set(catch_unwind(AssertUnwindSafe(|| poll(&mut context))).is_err());
        })
        .unwrap();
        assert!(poll(&mut Context::from_waker(Waker::noop())));
        assert!(rejected.get());
        assert!(!has_jobs());
        assert_eq!(Rc::strong_count(&rejected), 1);
    }
}
