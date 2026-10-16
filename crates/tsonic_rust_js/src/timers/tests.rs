use super::timer_id;

#[test]
fn a_public_timer_without_an_owner_panics_before_admitting_a_callback() {
    let calls = std::cell::Cell::new(0);
    let timers = super::new::<std::convert::Infallible>();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        super::set_timeout_callable(
            &timers,
            tsonic_rust_runtime::Callable::new(|()| {
                calls.set(calls.get() + 1);
                Ok(())
            }),
            0.0,
        )
    }))
    .expect_err("public timer admission must panic synchronously");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied());
    assert_eq!(
        message,
        Some("No JavaScript event loop is running on this thread.")
    );
    assert_eq!(calls.get(), 0);
    assert!(!timers.has_pending_work());
    assert!(!crate::promise_jobs::active());
}

#[test]
fn local_timer_context_retains_borrowed_callback_environment() {
    let calls = std::cell::Cell::new(0);
    let timers = super::new::<std::convert::Infallible>();
    super::set_timeout_callable(
        &timers,
        tsonic_rust_runtime::Callable::new(|()| {
            calls.set(calls.get() + 1);
            Ok(())
        }),
        0.0,
    )
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert!(timers.poll().unwrap());
    assert_eq!(calls.get(), 1);
    assert!(!timers.has_pending_work());
}

#[test]
fn local_timer_context_preserves_borrowed_failure_identity() {
    let failure = String::from("scoped timer failure");
    let timers = super::new::<&String>();
    super::set_timeout_callable(
        &timers,
        tsonic_rust_runtime::Callable::new(|()| Err(&failure)),
        0.0,
    )
    .unwrap();
    assert!(std::ptr::eq(timers.poll().unwrap_err(), &failure));
    assert!(!timers.has_pending_work());
}

#[test]
fn timer_identifiers_retain_native_precision() {
    assert_eq!(
        timer_id(9_007_199_254_740_993_u64),
        Some(9_007_199_254_740_993)
    );
    assert_eq!(timer_id(u64::MAX), Some(u64::MAX));
    assert_eq!(timer_id(7_i32), Some(7));
    assert_eq!(timer_id(7.0), Some(7));
    assert_eq!(timer_id(7.5), None);
    assert_eq!(timer_id(-1_i64), None);
    assert_eq!(timer_id(0_u64), None);
    assert_eq!(timer_id(f64::NAN), None);
    assert_eq!(timer_id(f64::INFINITY), None);
    assert_eq!(timer_id(u64::MAX as f64), None);
}

#[test]
fn selected_js_timer_driver_keeps_non_display_non_send_error_identity() {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use tsonic_rust_runtime::{dispatch::DispatchEnd, TsonicError};
    enum Failure {
        Native(TsonicError),
        Source(Rc<Cell<i64>>),
    }
    impl From<TsonicError> for Failure {
        fn from(value: TsonicError) -> Self {
            Self::Native(value)
        }
    }
    let timers = new::<Failure>();
    let original = Rc::new(Cell::new(9_007_199_254_740_993));
    let captured = Rc::clone(&original);
    set_timeout_callable(
        &timers,
        Callable::new(move |()| Err(Failure::Source(Rc::clone(&captured)))),
        0.0,
    )
    .unwrap();
    let observed = Rc::new(Cell::new(false));
    let recorded = Rc::clone(&observed);
    set_timeout_callable(
        &timers,
        Callable::new(move |()| {
            recorded.set(true);
            Ok(())
        }),
        0.0,
    )
    .unwrap();
    let contexts = tsonic_rust_runtime::dispatch::prepend(&timers, DispatchEnd::<Failure>::new());
    match crate::event_loop::run_with_contexts(&contexts).expect_err("original source timer error")
    {
        Failure::Source(value) => {
            assert!(Rc::ptr_eq(&value, &original));
            assert_eq!(value.get(), 9_007_199_254_740_993);
        }
        Failure::Native(error) => panic!("unexpected native failure: {error}"),
    }
    assert!(!observed.get());
    assert!(crate::event_loop::run_with_contexts(&contexts).is_ok());
    assert!(observed.get());
    assert!(!timers.has_pending_work());
}
