use std::time::Duration;

use crate::number::NativeNumberPredicate;
use crate::{JsError, JsResult};
use num_traits::ToPrimitive;
use tsonic_rust_runtime::dispatch::{DispatchContexts, DispatchPhase};
use tsonic_rust_runtime::timer_queue::{TimerContext, TimerQueueError};
use tsonic_rust_runtime::{Callable, TsonicResult};

pub const fn new<TError>() -> TimerContext<Callable<(), Result<(), TError>>> {
    TimerContext::new(DispatchPhase::JsTimers)
}

thread_local! {
    static DEFAULT_TIMERS: TimerContext<Callable<(), TsonicResult<()>>> = const { new() };
}

pub fn with_default<TOutput>(
    operation: impl FnOnce(&TimerContext<Callable<(), TsonicResult<()>>>) -> TOutput,
) -> TOutput {
    DEFAULT_TIMERS.with(operation)
}

pub fn set_timeout_callable<TError>(
    timers: &TimerContext<Callable<(), Result<(), TError>>>,
    callback: Callable<(), Result<(), TError>>,
    delay_ms: f64,
) -> JsResult<u64> {
    schedule_callback(timers, callback, normalized_delay(delay_ms), false)
}

pub fn set_interval_callable<TError>(
    timers: &TimerContext<Callable<(), Result<(), TError>>>,
    callback: Callable<(), Result<(), TError>>,
    delay_ms: f64,
) -> JsResult<u64> {
    schedule_callback(timers, callback, normalized_delay(delay_ms).max(1), true)
}

pub fn clear_timeout<TError, Value: Copy + NativeNumberPredicate + ToPrimitive>(
    timers: &TimerContext<Callable<(), Result<(), TError>>>,
    value: Value,
) {
    if let Some(id) = timer_id(value) {
        timers.cancel(id);
    }
}

pub fn clear_interval<TError, Value: Copy + NativeNumberPredicate + ToPrimitive>(
    timers: &TimerContext<Callable<(), Result<(), TError>>>,
    id: Value,
) {
    clear_timeout(timers, id);
}

pub fn run_timers() -> TsonicResult<()> {
    crate::event_loop::run_event_loop()
}

pub fn has_timers() -> bool {
    with_default(TimerContext::has_pending_work)
}

pub fn next_timer_delay() -> Option<Duration> {
    with_default(DispatchContexts::next_delay)
}

fn schedule_callback<TError>(
    timers: &TimerContext<Callable<(), Result<(), TError>>>,
    callback: Callable<(), Result<(), TError>>,
    delay_ms: u64,
    interval: bool,
) -> JsResult<u64> {
    timers
        .schedule_with(
            Duration::from_millis(delay_ms),
            interval,
            true,
            false,
            || callback,
        )
        .map(|handle| handle.id())
        .map_err(timer_error)
}

fn timer_error(error: TimerQueueError) -> JsError {
    crate::range_error(&error.to_string())
}

pub fn poll_timers() -> TsonicResult<bool> {
    with_default(TimerContext::poll)
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
mod tests;
