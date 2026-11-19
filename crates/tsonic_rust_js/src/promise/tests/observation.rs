use super::*;

fn externally_settled_promise() -> (JsPromise<i32>, PromiseResolve<i32>, PromiseReject) {
    let callbacks = Rc::new(RefCell::new(None));
    let captured = Rc::clone(&callbacks);
    let promise = JsPromise::create(Callable::new(move |(resolve, reject)| {
        captured.replace(Some((resolve, reject)));
        Ok(())
    }));
    let (resolve, reject) = callbacks.borrow_mut().take().expect("the executor ran");
    (promise, resolve, reject)
}

#[test]
fn pending_observation_without_an_owner_panics_before_mutating_the_state() {
    let promise = JsPromise::<i32>::pending();
    let retained = promise.clone();
    assert_missing_owner(
        catch_unwind(AssertUnwindSafe(|| promise.observe_discarded()))
            .expect_err("an unfinished Promise requires its observation owner immediately"),
    );
    assert!(
        matches!(
            &*retained.state.borrow(),
            PromiseState::Pending {
                observation_owner: None,
                ..
            }
        ),
        "missing ownership must not mark the pending state as observed"
    );
    assert!(
        take_reports().is_empty(),
        "no report is admitted before ownership"
    );
}

#[test]
fn rejected_observation_without_an_owner_panics_before_marking_or_reporting() {
    let promise = JsPromise::<i32>::rejected(JsValue::String(String::from("no owner")));
    let retained = promise.clone();
    assert_missing_owner(
        catch_unwind(AssertUnwindSafe(|| promise.observe_discarded()))
            .expect_err("a rejected Promise requires its observation owner immediately"),
    );
    assert!(
        matches!(
            &*retained.state.borrow(),
            PromiseState::Rejected {
                reported: false,
                ..
            }
        ),
        "missing ownership must not mark the rejection as reported"
    );
    assert!(
        take_reports().is_empty(),
        "missing ownership reports no source reason"
    );
}

#[test]
fn pending_observation_inside_its_owner_marks_once_without_enqueuing_work() {
    let promise = JsPromise::<i32>::pending();
    let retained = promise.clone();
    let ((), status) = run_entry_with_contexts(
        || {
            promise.observe_discarded();
            assert!(
                matches!(
                    &*retained.state.borrow(),
                    PromiseState::Pending { observation_owner: Some(owner), .. }
                        if Some(*owner) == crate::promise_jobs::current_owner()
                ),
                "the owned pending Promise is observed"
            );
            assert!(
                !crate::promise_jobs::has_ready(),
                "observation itself admits no task"
            );
            retained.settle(Ok(31));
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!status.exit_requested());
    assert!(take_reports().is_empty());
    let mut result = std::pin::pin!(retained.into_result());
    assert!(matches!(
        result
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Ok(31))
    ));
}

#[test]
fn fulfilled_observation_without_an_owner_needs_no_owner_or_report() {
    JsPromise::resolved(31).observe_discarded();
    assert!(!crate::promise_jobs::active());
    assert!(!crate::promise_jobs::has_ready());
    assert!(take_reports().is_empty());
}

#[test]
fn pending_observation_while_its_owner_closes_preserves_shutdown_status() {
    let owner = crate::promise_jobs::Owner::enter(Waker::noop().clone()).unwrap();
    let promise = JsPromise::<i32>::pending();
    let retained = promise.clone();
    owner.stop_admission();
    assert!(crate::promise_jobs::active());
    assert!(crate::promise_jobs::ensure_running().is_err());
    promise.observe_discarded();
    assert!(
        matches!(
            &*retained.state.borrow(),
            PromiseState::Pending { observation_owner: Some(owner), .. }
                if Some(*owner) == crate::promise_jobs::current_owner()
        ),
        "the closing owner still owns the observation"
    );
    assert!(!crate::promise_jobs::has_ready());
    retained.settle(Err(JsValue::Bool(false)));
    assert!(crate::promise_jobs::exit_requested());
    assert_eq!(take_reports(), "false\n");
    drop(owner);
    assert_eq!(crate::promise_jobs::take_exit(), Some(1));
    assert!(!crate::promise_jobs::active());
}

#[test]
fn repeated_rejected_observation_still_requires_a_live_owner() {
    let promise = JsPromise::<i32>::rejected(JsValue::Bool(false));
    let retained = promise.clone();
    let ((), status) =
        run_entry_with_contexts(|| promise.observe_discarded(), no_contexts()).unwrap();
    assert!(status.exit_requested());
    assert_eq!(take_reports(), "false\n");
    assert_missing_owner(
        catch_unwind(AssertUnwindSafe(|| retained.observe_discarded()))
            .expect_err("a previously reported rejection is not a fulfilled Promise"),
    );
    assert!(
        take_reports().is_empty(),
        "the failed observation produces no second report"
    );
}

#[test]
fn a_closed_owner_observation_cannot_report_a_later_external_rejection() {
    let (promise, _resolve, reject) = externally_settled_promise();
    let retained = promise.clone();
    let ((), status) =
        run_entry_with_contexts(|| promise.observe_discarded(), no_contexts()).unwrap();
    assert!(!status.exit_requested());
    let reason = JsValue::from_error(MutableJsError::error("late external rejection"));
    reject.call((reason.clone(),)).unwrap();
    assert!(!crate::promise_jobs::active());
    assert!(!crate::promise_jobs::has_ready());
    assert!(take_reports().is_empty());
    assert!(tsonic_rust_runtime::block_on(retained.into_result())
        .unwrap_err()
        .strict_equal(&reason));
}

#[test]
fn a_closed_owner_observation_cannot_report_inside_the_next_loop_generation() {
    let (promise, _resolve, reject) = externally_settled_promise();
    let retained = promise.clone();
    let ((), first_status) =
        run_entry_with_contexts(|| promise.observe_discarded(), no_contexts()).unwrap();
    assert!(!first_status.exit_requested());
    let reason = JsValue::from_error(MutableJsError::error("next generation rejection"));
    let ((), second_status) = run_entry_with_contexts(
        || {
            reject.call((reason.clone(),)).unwrap();
            assert!(!crate::promise_jobs::has_ready());
            assert!(take_reports().is_empty());
        },
        no_contexts(),
    )
    .unwrap();
    assert!(!second_status.exit_requested());
    assert!(take_reports().is_empty());
    assert!(tsonic_rust_runtime::block_on(retained.into_result())
        .unwrap_err()
        .strict_equal(&reason));
}

#[test]
fn a_pending_promise_can_be_explicitly_observed_by_the_next_loop_generation() {
    let (promise, _resolve, reject) = externally_settled_promise();
    let retained = promise.clone();
    let ((), first_status) =
        run_entry_with_contexts(|| promise.observe_discarded(), no_contexts()).unwrap();
    assert!(!first_status.exit_requested());
    let ((), second_status) = run_entry_with_contexts(
        || {
            retained.observe_discarded();
            assert!(!crate::promise_jobs::has_ready());
            reject.call((JsValue::Bool(false),)).unwrap();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(second_status.exit_requested());
    assert_eq!(take_reports(), "false\n");
}

#[test]
fn a_closed_owner_observation_does_not_restrict_later_external_fulfillment() {
    let (promise, resolve, _reject) = externally_settled_promise();
    let retained = promise.clone();
    let ((), status) =
        run_entry_with_contexts(|| promise.observe_discarded(), no_contexts()).unwrap();
    assert!(!status.exit_requested());
    resolve.call((PromiseResolution::Value(31),)).unwrap();
    assert_eq!(
        tsonic_rust_runtime::block_on(retained.into_result()).unwrap(),
        31
    );
    assert!(!crate::promise_jobs::active());
    assert!(take_reports().is_empty());
}
