use std::cell::Cell;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::task::Waker;
use tsonic_rust_js::event_loop::{block_on_with_driver, run_with_driver, EventLoopDriver};
use tsonic_rust_runtime::dispatch_queue::{TaskBudget, TaskQueue};

struct Failure(Rc<Cell<i64>>);

struct Driver {
    tasks: TaskQueue<Failure>,
    wake_failure: Option<Failure>,
    wait_failure: Option<Failure>,
}

impl Driver {
    fn new() -> Self {
        Self {
            tasks: TaskQueue::new(TaskBudget::new(NonZeroUsize::new(4).unwrap())),
            wake_failure: None,
            wait_failure: None,
        }
    }
}

impl EventLoopDriver for Driver {
    type Error = Failure;

    fn poll(&mut self) -> Result<bool, Failure> {
        self.tasks.poll_ready()
    }

    fn has_work(&self) -> bool {
        self.tasks.front_ticket().is_some()
    }

    fn wait(&mut self) -> Result<(), Failure> {
        Err(self
            .wait_failure
            .take()
            .expect("exact native waiting failure"))
    }

    fn waker(&mut self) -> Result<Waker, Failure> {
        match self.wake_failure.take() {
            Some(error) => Err(error),
            None => Ok(Waker::noop().clone()),
        }
    }
}

#[test]
fn driven_callbacks_preserve_native_failure_identity_and_pending_work() {
    let mut driver = Driver::new();
    let payload = Rc::new(Cell::new(9_007_199_254_740_993));
    let retained = payload.clone();
    let observed = Rc::new(Cell::new(0));
    let completed = observed.clone();
    driver
        .tasks
        .enqueue(move || Err(Failure(retained)))
        .unwrap();
    driver
        .tasks
        .enqueue(move || {
            completed.set(7);
            Ok(())
        })
        .unwrap();
    let failure = run_with_driver(&mut driver)
        .err()
        .expect("exact callback failure");
    assert!(Rc::ptr_eq(&failure.0, &payload));
    assert_eq!(failure.0.get(), 9_007_199_254_740_993);
    assert_eq!(observed.get(), 0);
    assert!(driver.has_work());
    assert_eq!(run_with_driver(&mut driver).ok(), Some(()));
    assert_eq!(observed.get(), 7);
    assert!(!driver.has_work());
}

#[test]
fn root_future_and_native_wake_errors_do_not_require_runtime_error_conversion() {
    let mut driver = Driver::new();
    assert_eq!(block_on_with_driver(async { 7 }, &mut driver).ok(), Some(7));
    let payload = Rc::new(Cell::new(9_007_199_254_740_993));
    driver.wake_failure = Some(Failure(payload.clone()));
    let failure = block_on_with_driver(async { 8 }, &mut driver)
        .err()
        .expect("exact wake failure");
    assert!(Rc::ptr_eq(&failure.0, &payload));
    assert_eq!(failure.0.get(), 9_007_199_254_740_993);
}

#[test]
fn native_wait_errors_preserve_their_selected_type_and_payload() {
    let mut driver = Driver::new();
    let payload = Rc::new(Cell::new(9_007_199_254_740_995));
    driver.wait_failure = Some(Failure(payload.clone()));
    let failure = block_on_with_driver(std::future::pending::<()>(), &mut driver)
        .err()
        .expect("exact native wait failure");
    assert!(Rc::ptr_eq(&failure.0, &payload));
    assert_eq!(failure.0.get(), 9_007_199_254_740_995);
}
