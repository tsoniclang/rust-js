use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tsonic_rust_runtime::{Callable, JsError, TsonicError, TsonicResult};

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

type TimerCallback = Rc<RefCell<Box<dyn FnMut() -> TsonicResult<()>>>>;

struct TimerEntry {
    callback: TimerCallback,
    delay: Duration,
    due: Instant,
    interval: bool,
}

thread_local! {
    static TIMERS: RefCell<BTreeMap<u64, TimerEntry>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn set_timeout_callable<E>(callback: Callable<(), Result<(), E>>, delay_ms: f64) -> f64
where
    E: std::fmt::Display + 'static,
{
    schedule_callback(callback, normalized_delay(delay_ms), false)
}

pub fn set_interval_callable<E>(callback: Callable<(), Result<(), E>>, delay_ms: f64) -> f64
where
    E: std::fmt::Display + 'static,
{
    schedule_callback(callback, normalized_delay(delay_ms).max(1), true)
}

pub fn clear_timeout(value: f64) {
    if let Some(timer_id) = timer_id(value) {
        TIMERS.with_borrow_mut(|timers| {
            timers.remove(&timer_id);
        });
    }
}

pub fn clear_interval(timer_id: f64) {
    clear_timeout(timer_id);
}

pub fn run_timers() -> TsonicResult<()> {
    loop {
        let next_delay = next_timer_delay();
        let Some(delay) = next_delay else {
            return Ok(());
        };
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
        poll_timers()?;
    }
}

pub fn has_timers() -> bool {
    TIMERS.with_borrow(|timers| !timers.is_empty())
}

pub fn next_timer_delay() -> Option<Duration> {
    TIMERS.with_borrow(|timers| {
        let now = Instant::now();
        timers
            .values()
            .map(|entry| entry.due.saturating_duration_since(now))
            .min()
    })
}

fn schedule_callback<E>(callback: Callable<(), Result<(), E>>, delay_ms: u64, interval: bool) -> f64
where
    E: std::fmt::Display + 'static,
{
    let callback = Box::new(move || {
        callback
            .call(())
            .map_err(|error| TsonicError::from(JsError::error(&error.to_string())))
    });
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
    let delay = Duration::from_millis(delay_ms);
    TIMERS.with_borrow_mut(|timers| {
        timers.insert(
            id,
            TimerEntry {
                callback: Rc::new(RefCell::new(callback)),
                delay,
                due: Instant::now() + delay,
                interval,
            },
        );
    });
    id as f64
}

pub fn poll_timers() -> TsonicResult<bool> {
    let now = Instant::now();
    let callbacks = TIMERS.with_borrow_mut(|timers| {
        let due = timers
            .iter()
            .filter_map(|(id, entry)| (entry.due <= now).then_some(*id))
            .collect::<Vec<_>>();
        let mut callbacks = Vec::with_capacity(due.len());
        for id in due {
            let Some(entry) = timers.get_mut(&id) else {
                continue;
            };
            callbacks.push(Rc::clone(&entry.callback));
            if entry.interval {
                entry.due = now + entry.delay;
            } else {
                timers.remove(&id);
            }
        }
        callbacks
    });
    let did_work = !callbacks.is_empty();
    for callback in callbacks {
        callback.borrow_mut()()?;
    }
    Ok(did_work)
}

fn normalized_delay(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else {
        value.trunc().min(u64::MAX as f64) as u64
    }
}

fn timer_id(value: f64) -> Option<u64> {
    (value.is_finite() && value >= 1.0 && value.fract() == 0.0 && value <= u64::MAX as f64)
        .then_some(value as u64)
}
