use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::number::NativeNumberPredicate;
use num_traits::ToPrimitive;
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

pub fn set_timeout_callable<E>(callback: Callable<(), Result<(), E>>, delay_ms: f64) -> u64
where
    E: std::fmt::Display + 'static,
{
    schedule_callback(callback, normalized_delay(delay_ms), false)
}

pub fn set_interval_callable<E>(callback: Callable<(), Result<(), E>>, delay_ms: f64) -> u64
where
    E: std::fmt::Display + 'static,
{
    schedule_callback(callback, normalized_delay(delay_ms).max(1), true)
}

pub fn clear_timeout<Value: Copy + NativeNumberPredicate + ToPrimitive>(value: Value) {
    if let Some(timer_id) = timer_id(value) {
        TIMERS.with_borrow_mut(|timers| {
            timers.remove(&timer_id);
        });
    }
}

pub fn clear_interval<Value: Copy + NativeNumberPredicate + ToPrimitive>(timer_id: Value) {
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

fn schedule_callback<E>(callback: Callable<(), Result<(), E>>, delay_ms: u64, interval: bool) -> u64
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
    id
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

fn timer_id<Value: Copy + NativeNumberPredicate + ToPrimitive>(value: Value) -> Option<u64> {
    if !value.native_is_integer() {
        return None;
    }
    value.to_u64().filter(|value| *value != 0)
}

#[cfg(test)]
mod tests {
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
}
