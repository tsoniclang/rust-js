use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;

use tsonic_rust_js::event_loop::{block_on, run_entry_with_contexts};
use tsonic_rust_js::promise::JsPromise;
use tsonic_rust_js::JsValue;
use tsonic_rust_runtime::dispatch::DispatchEnd;
use tsonic_rust_runtime::{Callable, JsError, TsonicError};

const STOPPED_ON_REQUESTED_EXIT: &str = "The event loop stopped on a requested exit.";
const NO_OWNER_CHILD: &str = "TSONIC_K12R_NO_OWNER_REPORT_CHILD";

fn no_contexts() -> DispatchEnd<TsonicError> {
    DispatchEnd::<TsonicError>::new()
}

fn discard_rejection(message: &'static str) {
    JsPromise::<i32>::from_fallible_factory(move || async move {
        Err(JsValue::from_error(JsError::error(message)))
    })
    .observe_discarded();
}

fn two_due_timers(first_requests_exit: bool) -> (bool, bool) {
    let second = Rc::new(Cell::new(false));
    let observed = Rc::clone(&second);
    let ((), status) = run_entry_with_contexts(
        move || {
            tsonic_rust_js::timers::with_default(|timers| {
                tsonic_rust_js::timers::set_timeout_callable(
                    timers,
                    Callable::new(move |()| {
                        if first_requests_exit {
                            discard_rejection("first timer");
                        }
                        Ok(())
                    }),
                    0.0,
                )
            })
            .expect("first timer under the owner");
            tsonic_rust_js::timers::with_default(|timers| {
                tsonic_rust_js::timers::set_timeout_callable(
                    timers,
                    Callable::new(move |()| {
                        observed.set(true);
                        Ok(())
                    }),
                    0.0,
                )
            })
            .expect("second timer under the owner");
        },
        no_contexts(),
    )
    .unwrap();
    (status.exit_requested(), second.get())
}

#[test]
fn a_requested_exit_stops_the_loop_before_a_second_due_timer() {
    let (exit_requested, second_ran) = two_due_timers(true);
    assert!(exit_requested, "the first timer's report requested exit");
    assert!(
        !second_ran,
        "the second due timer ran after the first requested exit"
    );
}

#[test]
fn without_a_requested_exit_both_due_timers_run() {
    let (exit_requested, second_ran) = two_due_timers(false);
    assert!(!exit_requested);
    assert!(second_ran);
}

#[test]
fn block_on_reports_a_requested_exit_after_its_root_completed() {
    let result = block_on(async {
        discard_rejection("root");
        5
    });
    match result {
        Ok(value) => panic!("block_on returned {value} after a requested exit"),
        Err(error) => assert!(
            error.source_error().message() == STOPPED_ON_REQUESTED_EXIT,
            "{error}"
        ),
    }
}

struct DiscardOnDrop;

impl Drop for DiscardOnDrop {
    fn drop(&mut self) {
        JsPromise::<i32>::rejected(JsValue::String(String::from("closing"))).observe_discarded();
    }
}

#[test]
fn a_rejection_observed_while_the_owner_closes_requests_exit_one() {
    let ((), status) = run_entry_with_contexts(
        || {
            drop(JsPromise::<()>::from_infallible_factory(|| async {
                let _guard = DiscardOnDrop;
                std::future::pending::<()>().await;
            }));
        },
        no_contexts(),
    )
    .unwrap();
    assert!(
        status.exit_requested(),
        "a report written during shutdown requested no exit"
    );
}

#[test]
fn a_rejection_discarded_without_an_owner_child() {
    if std::env::var_os(NO_OWNER_CHILD).is_none() {
        return;
    }
    JsPromise::<i32>::rejected(JsValue::String(String::from("no owner"))).observe_discarded();
    println!("the caller continued after the report");
}

#[test]
fn a_rejection_discarded_without_an_owner_is_reported_and_exits_one() {
    let output = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "requested_exit_witnesses::a_rejection_discarded_without_an_owner_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(NO_OWNER_CHILD, "1")
        .output()
        .expect("the child test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1),
        "child status {:?}; stderr: {stderr}",
        output.status
    );
    assert!(
        stderr.lines().any(|line| line == "no owner"),
        "stderr: {stderr}"
    );
    assert!(!stdout.contains("the caller continued"), "stdout: {stdout}");
}
