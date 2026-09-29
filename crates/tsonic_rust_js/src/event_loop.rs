use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::Duration;

use tsonic_rust_runtime::TsonicResult;

pub trait EventLoopDriver {
    fn poll(&mut self) -> TsonicResult<bool>;
    fn has_work(&self) -> bool;
    fn wait(&mut self, timer_delay: Option<Duration>) -> TsonicResult<()>;
    fn waker(&mut self) -> TsonicResult<Waker>;
}

struct LoopWake {
    ready: AtomicBool,
    driver: Waker,
}

impl Wake for LoopWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.ready.store(true, Ordering::Release);
        self.driver.wake_by_ref();
    }
}

struct ThreadWake(Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

struct TimerDriver;

impl EventLoopDriver for TimerDriver {
    fn poll(&mut self) -> TsonicResult<bool> {
        Ok(false)
    }

    fn has_work(&self) -> bool {
        false
    }

    fn wait(&mut self, timer_delay: Option<Duration>) -> TsonicResult<()> {
        match timer_delay {
            Some(delay) => thread::park_timeout(delay),
            None => thread::park(),
        }
        Ok(())
    }

    fn waker(&mut self) -> TsonicResult<Waker> {
        Ok(Waker::from(Arc::new(ThreadWake(thread::current()))))
    }
}

pub fn block_on<Output>(future: impl Future<Output = Output>) -> TsonicResult<Output> {
    block_on_with_driver(future, &mut TimerDriver)
}

pub fn run_event_loop() -> TsonicResult<()> {
    run_with_driver(&mut TimerDriver)
}

pub fn block_on_with_driver<Output>(
    future: impl Future<Output = Output>,
    driver: &mut impl EventLoopDriver,
) -> TsonicResult<Output> {
    let mut future = pin!(future);
    Ok(drive(Some(future.as_mut()), driver)?.expect("a driven root future returns its output"))
}

pub fn run_with_driver(driver: &mut impl EventLoopDriver) -> TsonicResult<()> {
    drive::<std::future::Pending<()>>(None, driver).map(|_| ())
}

fn drive<Root: Future>(
    mut root: Option<Pin<&mut Root>>,
    driver: &mut impl EventLoopDriver,
) -> TsonicResult<Option<Root::Output>> {
    if root.is_none()
        && !driver.has_work()
        && !crate::timers::has_timers()
        && !crate::promise_jobs::has_jobs()
    {
        return Ok(None);
    }
    let wake = Arc::new(LoopWake {
        ready: AtomicBool::new(true),
        driver: driver.waker()?,
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);
    loop {
        let timer_work = crate::timers::poll_timers()?;
        let driver_work = driver.poll()?;
        let mut job_work = false;
        if wake.ready.swap(false, Ordering::AcqRel) {
            if let Some(future) = &mut root {
                if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                    return Ok(Some(output));
                }
            }
            job_work = crate::promise_jobs::poll(&mut context);
        }
        if root.is_none()
            && !driver.has_work()
            && !crate::timers::has_timers()
            && !wake.ready.load(Ordering::Acquire)
        {
            return Ok(None);
        }
        if !timer_work && !driver_work && !job_work && !wake.ready.load(Ordering::Acquire) {
            driver.wait(crate::timers::next_timer_delay())?;
        }
    }
}
