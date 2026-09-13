use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::errors::{range_error, JsResult};

#[derive(Debug)]
struct Waiter {
    id: u64,
    offset: usize,
    notified: bool,
}

#[derive(Debug)]
pub(crate) struct SharedState {
    pub(crate) bytes: Vec<u8>,
    waiters: VecDeque<Waiter>,
    next_waiter: u64,
}

#[derive(Debug)]
pub struct SharedBufferStorage {
    state: Mutex<SharedState>,
    changed: Condvar,
}

impl SharedBufferStorage {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self {
            state: Mutex::new(SharedState {
                bytes,
                waiters: VecDeque::new(),
                next_waiter: 0,
            }),
            changed: Condvar::new(),
        }
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, SharedState> {
        self.state.lock().expect("shared buffer lock poisoned")
    }

    pub(crate) fn wait_i32(
        &self,
        offset: usize,
        expected: i32,
        timeout: f64,
    ) -> JsResult<&'static str> {
        let duration = if timeout.is_nan() || timeout == f64::INFINITY {
            None
        } else {
            Some(timeout.max(0.0) / 1000.0)
        };
        let order = crate::atomics::ATOMIC_ORDER
            .lock()
            .expect("atomic order lock poisoned");
        let mut state = self.lock();
        let bytes = state
            .bytes
            .get(offset..offset + 4)
            .ok_or_else(|| range_error("atomic wait address exceeds the shared buffer"))?;
        if i32::from_le_bytes(bytes.try_into().expect("checked Int32 width")) != expected {
            return Ok("not-equal");
        }
        let id = state.next_waiter;
        state.next_waiter = id
            .checked_add(1)
            .ok_or_else(|| range_error("atomic waiter identity space exhausted"))?;
        state.waiters.push_back(Waiter {
            id,
            offset,
            notified: false,
        });
        drop(order);
        let started = Instant::now();
        let outcome = loop {
            if state
                .waiters
                .iter()
                .any(|waiter| waiter.id == id && waiter.notified)
            {
                break "ok";
            }
            state = match duration {
                None => self
                    .changed
                    .wait(state)
                    .expect("shared buffer lock poisoned"),
                Some(duration) => {
                    let remaining = duration - started.elapsed().as_secs_f64();
                    if remaining <= 0.0 {
                        break "timed-out";
                    }
                    self.changed
                        .wait_timeout(state, Duration::from_secs_f64(remaining.min(86_400.0)))
                        .expect("shared buffer lock poisoned")
                        .0
                }
            };
        };
        state.waiters.retain(|waiter| waiter.id != id);
        Ok(outcome)
    }

    pub(crate) fn notify(&self, offset: usize, count: f64) -> usize {
        let count = if count.is_nan() || count <= 0.0 {
            0
        } else {
            count.trunc() as usize
        };
        let mut state = self.lock();
        let mut notified = 0;
        for waiter in &mut state.waiters {
            if notified == count {
                break;
            }
            if waiter.offset == offset && !waiter.notified {
                waiter.notified = true;
                notified += 1;
            }
        }
        if notified > 0 {
            self.changed.notify_all();
        }
        notified
    }
}
