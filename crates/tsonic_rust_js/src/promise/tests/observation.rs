use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn assert_missing_owner(failure: Box<dyn std::any::Any + Send>) {
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied());
    assert_eq!(message, Some(NO_LOOP));
    assert!(!crate::promise_jobs::active());
    assert!(!crate::promise_jobs::has_ready());
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
                observed: false,
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
                    PromiseState::Pending { observed: true, .. }
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
