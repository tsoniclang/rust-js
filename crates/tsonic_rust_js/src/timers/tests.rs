use super::timer_id;

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
    match crate::event_loop::run_with_contexts(&contexts)
        .err()
        .expect("original source timer error")
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
