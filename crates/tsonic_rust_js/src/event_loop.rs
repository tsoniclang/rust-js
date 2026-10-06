use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

use tsonic_rust_runtime::dispatch::{DispatchContexts, DispatchEnd, DispatchPhase};
use tsonic_rust_runtime::TsonicResult;

pub trait EventLoopDriver {
    type Error;

    fn poll(&mut self) -> Result<bool, Self::Error>;
    fn has_work(&self) -> bool;
    fn wait(&mut self) -> Result<(), Self::Error>;
    fn waker(&mut self) -> Result<Waker, Self::Error>;
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

struct TimerDriver<TContexts> {
    contexts: TContexts,
}

impl<TContexts: DispatchContexts> EventLoopDriver for TimerDriver<TContexts>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    type Error = TContexts::Error;

    fn poll(&mut self) -> Result<bool, Self::Error> {
        crate::timers::with_default(|native| {
            tsonic_rust_runtime::dispatch::poll_phase(
                &tsonic_rust_runtime::dispatch::prepend(native, &self.contexts),
                DispatchPhase::JsTimers,
            )
        })
    }

    fn has_work(&self) -> bool {
        crate::timers::has_timers() || self.contexts.has_work()
    }

    fn wait(&mut self) -> Result<(), Self::Error> {
        let native = crate::timers::next_timer_delay();
        let selected = self.contexts.next_delay();
        let delay = match (native, selected) {
            (Some(native), Some(selected)) => Some(native.min(selected)),
            (Some(delay), None) | (None, Some(delay)) => Some(delay),
            (None, None) => None,
        };
        match delay {
            Some(delay) => thread::park_timeout(delay),
            None => thread::park(),
        }
        Ok(())
    }

    fn waker(&mut self) -> Result<Waker, Self::Error> {
        Ok(Waker::from(Arc::new(ThreadWake(thread::current()))))
    }
}

pub fn block_on<Output>(future: impl Future<Output = Output>) -> TsonicResult<Output> {
    block_on_with_contexts(
        future,
        DispatchEnd::<tsonic_rust_runtime::TsonicError>::new(),
    )
}

pub fn run_event_loop() -> TsonicResult<()> {
    run_with_contexts(DispatchEnd::<tsonic_rust_runtime::TsonicError>::new())
}

pub fn block_on_with_contexts<TOutput, TContexts: DispatchContexts>(
    future: impl Future<Output = TOutput>,
    contexts: TContexts,
) -> Result<TOutput, TContexts::Error>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    block_on_with_driver(future, &mut TimerDriver { contexts })
}

pub fn run_with_contexts<TContexts: DispatchContexts>(
    contexts: TContexts,
) -> Result<(), TContexts::Error>
where
    TContexts::Error: From<tsonic_rust_runtime::TsonicError>,
{
    run_with_driver(&mut TimerDriver { contexts })
}

pub fn block_on_with_driver<Output, Driver: EventLoopDriver>(
    future: impl Future<Output = Output>,
    driver: &mut Driver,
) -> Result<Output, Driver::Error> {
    let mut future = pin!(future);
    Ok(drive(Some(future.as_mut()), driver)?.expect("a driven root future returns its output"))
}

pub fn run_with_driver<Driver: EventLoopDriver>(driver: &mut Driver) -> Result<(), Driver::Error> {
    drive::<std::future::Pending<()>, Driver>(None, driver).map(|_| ())
}

fn drive<Root: Future, Driver: EventLoopDriver>(
    mut root: Option<Pin<&mut Root>>,
    driver: &mut Driver,
) -> Result<Option<Root::Output>, Driver::Error> {
    if root.is_none() && !driver.has_work() && !crate::promise_jobs::has_jobs() {
        return Ok(None);
    }
    let wake = Arc::new(LoopWake {
        ready: AtomicBool::new(true),
        driver: driver.waker()?,
    });
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);
    loop {
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
        if root.is_none() && !driver.has_work() && !wake.ready.load(Ordering::Acquire) {
            return Ok(None);
        }
        if !driver_work && !job_work && !wake.ready.load(Ordering::Acquire) {
            driver.wait()?;
        }
    }
}
