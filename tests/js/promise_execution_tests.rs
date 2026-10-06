use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use tsonic_rust_js::event_loop::{block_on, run_event_loop};
use tsonic_rust_js::promise::{JsPromise, PromiseReject, PromiseResolution, PromiseResolve};
use tsonic_rust_runtime::{Callable, JsError, TsonicError};

#[test]
fn executor_runs_eagerly_and_preserves_first_resolution_during_adoption() {
    let calls = Rc::new(Cell::new(0));
    let executor_calls = Rc::clone(&calls);
    let pending_resolve = Rc::new(RefCell::new(None::<PromiseResolve<u64>>));
    let captured_resolve = Rc::clone(&pending_resolve);
    let inner = JsPromise::create(Callable::new(move |(resolve, _)| {
        captured_resolve.replace(Some(resolve));
        Ok(())
    }));
    let outer = JsPromise::create(Callable::new(
        move |(resolve, reject): (PromiseResolve<u64>, PromiseReject)| {
            executor_calls.set(executor_calls.get() + 1);
            resolve.call((PromiseResolution::Promise(inner.clone()),))?;
            reject.call((JsError::error("late rejection").into(),))?;
            resolve.call((PromiseResolution::Value(0),))?;
            Err(JsError::error("late throw").into())
        },
    ));
    assert_eq!(calls.get(), 1);
    pending_resolve
        .borrow()
        .as_ref()
        .unwrap()
        .call((PromiseResolution::Value(9_007_199_254_740_993),))
        .unwrap();
    assert_eq!(
        block_on(outer.into_result()).unwrap().unwrap(),
        9_007_199_254_740_993
    );
}

#[test]
fn timer_settlement_wakes_a_borrowed_root_future() {
    let value = String::from("borrowed root");
    let promise = JsPromise::create(Callable::new(
        |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
            tsonic_rust_js::timers::with_default(|timers| {
                tsonic_rust_js::timers::set_timeout_callable(
                    timers,
                    Callable::new(move |()| resolve.call((PromiseResolution::Value(7),))),
                    1.0,
                )
            })
            .unwrap();
            Ok(())
        },
    ));
    let result = block_on(async {
        assert_eq!(value, "borrowed root");
        promise.into_result().await
    })
    .unwrap()
    .unwrap();
    assert_eq!(result, 7);
    assert_eq!(value, "borrowed root");
}

#[test]
fn discarded_continuations_run_and_can_enqueue_continuations() {
    let count = Rc::new(Cell::new(0));
    let first_count = Rc::clone(&count);
    drop(JsPromise::<i32>::resolved(3).then(
        Callable::new(move |(value,)| {
            first_count.set(value);
            let next_count = Rc::clone(&first_count);
            drop(JsPromise::<i32>::resolved(4).then(
                Callable::new(move |(next,)| {
                    next_count.set(next_count.get() + next);
                    Ok(())
                }),
                None,
            ));
            Ok(())
        }),
        None,
    ));
    assert_eq!(count.get(), 0);
    run_event_loop().unwrap();
    assert_eq!(count.get(), 7);
}

#[test]
fn continuation_failures_keep_error_identity_and_do_not_invoke_own_reject_handler() {
    let expected = JsError::error("callback failure");
    let thrown = expected.clone();
    let calls = Rc::new(Cell::new(0));
    let rejected_calls = Rc::clone(&calls);
    let promise = JsPromise::resolved(1).then(
        Callable::new(move |(_value,)| Err(TsonicError::from(thrown.clone()))),
        Some(Callable::new(move |(_error,)| {
            rejected_calls.set(rejected_calls.get() + 1);
            Ok(0)
        })),
    );
    let error = block_on(promise.into_result()).unwrap().unwrap_err();
    assert!(error.source_error().has_same_identity(&expected));
    assert_eq!(calls.get(), 0);
    let recovered = JsPromise::<i32>::rejected(error).catch(Callable::new(|(_reason,)| Ok(9)));
    assert_eq!(block_on(recovered.into_result()).unwrap().unwrap(), 9);
}

#[test]
fn asynchronous_continuation_adopts_the_selected_native_promise() {
    let result = JsPromise::<i32>::resolved(4).then_async(
        Callable::new(|(value,)| Ok(JsPromise::resolved(value + 5))),
        None,
    );
    assert_eq!(block_on(result.into_result()).unwrap().unwrap(), 9);
}

#[test]
fn self_resolution_rejects_and_does_not_leak_a_cycle() {
    let resolve = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
    let captured = Rc::clone(&resolve);
    let promise = JsPromise::create(Callable::new(move |(resolver, _)| {
        captured.replace(Some(resolver));
        Ok(())
    }));
    resolve
        .borrow_mut()
        .take()
        .unwrap()
        .call((PromiseResolution::Promise(promise.clone()),))
        .unwrap();
    let error = block_on(promise.into_result()).unwrap().unwrap_err();
    assert_eq!(
        error.source_error().kind(),
        tsonic_rust_runtime::JsErrorKind::TypeError
    );
}

#[test]
fn a_native_thread_wakes_the_root_without_busy_polling() {
    let state = Arc::new(Mutex::new((false, None::<std::task::Waker>)));
    let sender = Arc::clone(&state);
    let worker = std::thread::spawn(move || loop {
        let wake = {
            let mut state = sender.lock().unwrap();
            if state.1.is_some() {
                state.0 = true;
                state.1.take()
            } else {
                None
            }
        };
        if let Some(wake) = wake {
            wake.wake();
            break;
        }
        std::thread::yield_now();
    });
    let polls = Cell::new(0);
    block_on(poll_fn(|context| {
        polls.set(polls.get() + 1);
        let mut state = state.lock().unwrap();
        if state.0 {
            Poll::Ready(())
        } else {
            state.1 = Some(context.waker().clone());
            Poll::Pending
        }
    }))
    .unwrap();
    worker.join().unwrap();
    assert_eq!(polls.get(), 2);
}

#[derive(Clone, Debug)]
enum ProjectError {
    Builtin(JsError),
    Authored(Rc<String>),
}

impl From<JsError> for ProjectError {
    fn from(error: JsError) -> Self {
        Self::Builtin(error)
    }
}

#[test]
fn project_owned_rejections_keep_payload_and_identity_in_continuations_and_combinators() {
    use tsonic_rust_js::promise::{promise_all_settled, promise_race, PromiseSettledResult};
    use tsonic_rust_js::JsArray;

    let identity = Rc::new(String::from("authored payload"));
    let rejected =
        JsPromise::<i32, ProjectError>::rejected(ProjectError::Authored(identity.clone()));
    let values = JsArray::from_dense(vec![rejected.clone(), JsPromise::resolved(3)]);
    let race = block_on(promise_race(&values).into_result()).unwrap();
    assert!(matches!(race, Err(ProjectError::Authored(value)) if Rc::ptr_eq(&identity, &value)));
    let settled = block_on(promise_all_settled(&values).into_result())
        .unwrap()
        .unwrap();
    assert!(
        matches!(settled.get(0), Some(PromiseSettledResult::Rejected(value))
        if matches!(&value.reason, ProjectError::Authored(reason) if Rc::ptr_eq(&identity, reason)))
    );
    let expected = Rc::clone(&identity);
    let recovered = rejected.catch(Callable::new(move |(reason,)| {
        assert!(matches!(reason, ProjectError::Authored(value) if Rc::ptr_eq(&expected, &value)));
        Ok(11)
    }));
    assert_eq!(block_on(recovered.into_result()).unwrap().unwrap(), 11);
    let builtin = ProjectError::from(JsError::error("native"));
    assert!(matches!(builtin, ProjectError::Builtin(error) if error.message() == "native"));
}

#[test]
fn discarded_finalizers_run_and_keep_the_original_settlement() {
    let runs = Rc::new(Cell::new(0));
    let captured = Rc::clone(&runs);
    drop(
        JsPromise::<i32>::resolved(7).finally(Callable::new(move |()| {
            captured.set(captured.get() + 1);
            Ok(())
        })),
    );
    assert_eq!(runs.get(), 0);
    run_event_loop().unwrap();
    assert_eq!(runs.get(), 1);
    assert_eq!(Rc::strong_count(&runs), 1);
}

#[derive(Default)]
struct WakeCount(std::sync::atomic::AtomicUsize);

impl std::task::Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn canceling_the_first_waiter_does_not_lose_the_shared_future_wakeup() {
    use std::future::Future;
    use std::task::{Context, Waker};

    let readiness = Rc::new(Cell::new(false));
    let stored_waker = Rc::new(RefCell::new(None::<Waker>));
    let future_ready = Rc::clone(&readiness);
    let future_waker = Rc::clone(&stored_waker);
    let promise = JsPromise::<i32>::from_infallible_factory(move || {
        poll_fn(move |context| {
            if future_ready.get() {
                Poll::Ready(19)
            } else {
                future_waker.replace(Some(context.waker().clone()));
                Poll::Pending
            }
        })
    });
    let first_wakes = Arc::new(WakeCount::default());
    let second_wakes = Arc::new(WakeCount::default());
    let first_waker = Waker::from(Arc::clone(&first_wakes));
    let second_waker = Waker::from(Arc::clone(&second_wakes));
    let mut first = Box::pin(promise.await_result());
    let mut second = Box::pin(promise.await_result());
    assert!(first
        .as_mut()
        .poll(&mut Context::from_waker(&first_waker))
        .is_pending());
    assert!(second
        .as_mut()
        .poll(&mut Context::from_waker(&second_waker))
        .is_pending());
    drop(first);
    readiness.set(true);
    stored_waker.borrow_mut().take().unwrap().wake();
    assert_eq!(first_wakes.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(second_wakes.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(matches!(
        second
            .as_mut()
            .poll(&mut Context::from_waker(&second_waker)),
        Poll::Ready(Ok(19))
    ));
    drop(second);
    assert_eq!(block_on(promise.into_result()).unwrap().unwrap(), 19);
}

#[test]
fn canceled_deferred_waiters_release_their_wakers_before_settlement() {
    use std::future::Future;
    use std::task::{Context, Waker};

    let resolver = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
    let captured = Rc::clone(&resolver);
    let promise = JsPromise::create(Callable::new(move |(resolve, _)| {
        captured.replace(Some(resolve));
        Ok(())
    }));
    let wakes = Arc::new(WakeCount::default());
    let waker = Waker::from(Arc::clone(&wakes));
    for _ in 0..1024 {
        let mut waiting = Box::pin(promise.await_result());
        assert!(waiting
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending());
        drop(waiting);
        assert_eq!(Arc::strong_count(&wakes), 2);
    }
    resolver
        .borrow_mut()
        .take()
        .unwrap()
        .call((PromiseResolution::Value(23),))
        .unwrap();
    assert_eq!(wakes.0.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(block_on(promise.into_result()).unwrap().unwrap(), 23);
}
