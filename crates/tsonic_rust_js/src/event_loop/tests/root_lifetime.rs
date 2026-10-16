use super::*;

#[derive(Clone, Copy)]
enum ExitCase {
    Ready,
    Requested,
    WaitError,
    EnterError,
    Unwind,
}

struct Root<'scope> {
    case: ExitCase,
    observed: &'scope RefCell<Option<(bool, bool, usize)>>,
    transitions: &'scope RefCell<Vec<(LoopLifecycle, bool)>>,
}

impl Future for Root<'_> {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<i32> {
        match self.case {
            ExitCase::Ready => Poll::Ready(31),
            ExitCase::Requested => {
                assert!(crate::promise_jobs::request_exit(7));
                Poll::Pending
            }
            ExitCase::WaitError => Poll::Pending,
            ExitCase::EnterError => panic!("an unentered root must not be polled"),
            ExitCase::Unwind => panic!("original root failure"),
        }
    }
}

impl Drop for Root<'_> {
    fn drop(&mut self) {
        assert!(self.observed.borrow().is_none(), "the root drops once");
        self.observed.replace(Some((
            crate::promise_jobs::active(),
            ensure_running().is_ok(),
            self.transitions.borrow().len(),
        )));
    }
}

fn assert_root_lifetime(case: ExitCase) {
    let observed = RefCell::new(None);
    let transitions = RefCell::new(Vec::new());
    let mut driver = LifecycleDriver {
        transitions: &transitions,
        fail_enter: matches!(case, ExitCase::EnterError),
        fail_wait: matches!(case, ExitCase::WaitError),
    };
    let result = catch_unwind(AssertUnwindSafe(|| {
        block_on_with_driver(
            Root {
                case,
                observed: &observed,
                transitions: &transitions,
            },
            &mut driver,
        )
    }));
    match case {
        ExitCase::Ready => {
            let (output, status) = result.unwrap().unwrap();
            assert_eq!(output, Some(31));
            assert!(!status.exit_requested());
        }
        ExitCase::Requested => {
            let (output, status) = result.unwrap().unwrap();
            assert!(output.is_none());
            assert_eq!(std::process::ExitCode::from(status), std::process::ExitCode::from(7));
        }
        ExitCase::WaitError => {
            assert_eq!(result.unwrap().unwrap_err().message(), "original wait error");
        }
        ExitCase::EnterError => {
            assert_eq!(result.unwrap().unwrap_err().message(), "original entry error");
        }
        ExitCase::Unwind => {
            let error = result.unwrap_err();
            assert_eq!(error.downcast_ref::<&str>().copied(), Some("original root failure"));
        }
    }
    assert_lifecycle(&transitions);
    assert_eq!(
        *observed.borrow(),
        Some((true, false, 1)),
        "the native root must drop under its closing owner before resource shutdown"
    );
}

#[test]
fn ready_root_drops_inside_its_closing_owner() {
    assert_root_lifetime(ExitCase::Ready);
}

#[test]
fn requested_exit_drops_the_pending_root_inside_its_closing_owner() {
    assert_root_lifetime(ExitCase::Requested);
}

#[test]
fn driver_error_drops_the_pending_root_inside_its_closing_owner() {
    assert_root_lifetime(ExitCase::WaitError);
}

#[test]
fn admission_error_drops_the_unpolled_root_inside_its_closing_owner() {
    assert_root_lifetime(ExitCase::EnterError);
}

#[test]
fn unwind_drops_the_root_inside_its_closing_owner() {
    assert_root_lifetime(ExitCase::Unwind);
}
