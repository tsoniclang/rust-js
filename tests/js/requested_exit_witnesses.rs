use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
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

fn discard_after_a_settled_reaction() {
    drop(JsPromise::resolved(0).then(
        Callable::new(|(value,): (i32,)| {
            discard_rejection("chained");
            Ok(value)
        }),
        None,
    ));
}

fn two_due_timers(first: fn()) -> (bool, bool) {
    let second = Rc::new(Cell::new(false));
    let observed = Rc::clone(&second);
    let ((), status) = run_entry_with_contexts(
        move || {
            tsonic_rust_js::timers::with_default(|timers| {
                tsonic_rust_js::timers::set_timeout_callable(
                    timers,
                    Callable::new(move |()| {
                        first();
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
    let (exit_requested, second_ran) = two_due_timers(|| discard_rejection("first timer"));
    assert!(exit_requested, "the first timer's report requested exit");
    assert!(
        !second_ran,
        "the second due timer ran after the first requested exit"
    );
}

#[test]
fn a_report_queued_by_a_reaction_stops_the_loop_before_a_second_due_timer() {
    let (exit_requested, second_ran) = two_due_timers(discard_after_a_settled_reaction);
    assert!(exit_requested, "the chained report requested exit");
    assert!(
        !second_ran,
        "the second due timer ran before the chained report"
    );
}

#[test]
fn a_report_queued_by_a_reaction_in_the_entry_stops_the_loop_before_a_zero_delay_timer() {
    let timer_ran = Rc::new(Cell::new(false));
    let observed = Rc::clone(&timer_ran);
    let ((), status) = run_entry_with_contexts(
        move || {
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
            .expect("timer under the owner");
            discard_after_a_settled_reaction();
        },
        no_contexts(),
    )
    .unwrap();
    assert!(status.exit_requested(), "the chained report requested exit");
    assert!(
        !timer_ran.get(),
        "the zero-delay timer ran before the chained report"
    );
}

#[test]
fn without_a_requested_exit_both_due_timers_run() {
    let (exit_requested, second_ran) = two_due_timers(|| {});
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

struct ReportingRoot {
    _report: DiscardOnDrop,
}

impl std::future::Future for ReportingRoot {
    type Output = i32;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<i32> {
        std::task::Poll::Ready(17)
    }
}

#[test]
fn a_rejection_from_root_destruction_returns_the_loop_status_instead_of_panicking() {
    let (output, status) = tsonic_rust_js::event_loop::block_on_with_contexts(
        ReportingRoot {
            _report: DiscardOnDrop,
        },
        no_contexts(),
    )
    .unwrap();
    assert_eq!(output, Some(17));
    assert!(status.exit_requested());
    assert_eq!(
        std::process::ExitCode::from(status),
        std::process::ExitCode::from(1)
    );
    assert!(tsonic_rust_js::event_loop::ensure_running().is_err());
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
    struct UnwindWitness(Rc<Cell<bool>>);

    impl Drop for UnwindWitness {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let unwound = Rc::new(Cell::new(false));
    let observed = Rc::clone(&unwound);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        let _guard = UnwindWitness(observed);
        JsPromise::<i32>::rejected(JsValue::String(String::from("no owner"))).observe_discarded();
        panic!("the discarded rejection returned instead of panicking");
    }))
    .expect_err("missing loop ownership must panic");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied());
    assert_eq!(
        message,
        Some("No JavaScript event loop is running on this thread.")
    );
    assert!(
        unwound.get(),
        "native destructors run during the ownership panic"
    );
    assert!(tsonic_rust_js::event_loop::ensure_running().is_err());
    println!("the native caller caught the ownership panic and its destructor ran");
}

#[test]
fn a_rejection_discarded_without_an_owner_reports_and_unwinds_without_exiting() {
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
        output.status.success(),
        "child status {:?}; stderr: {stderr}",
        output.status
    );
    assert!(
        !stderr.lines().any(|line| line == "no owner"),
        "a missing owner must reject observation before reporting its reason: {stderr}"
    );
    assert!(
        stdout.contains("the native caller caught the ownership panic and its destructor ran"),
        "stdout: {stdout}"
    );
    assert!(
        stderr.contains("No JavaScript event loop is running on this thread."),
        "stderr: {stderr}"
    );
}
