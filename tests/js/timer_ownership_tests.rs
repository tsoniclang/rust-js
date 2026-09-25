use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

fn record_allocation(bytes: usize) {
    ALLOCATIONS.with(|counter| {
        if let Some((count, total)) = counter.get() {
            counter.set(Some((count + 1, total + bytes)));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        unsafe { System.realloc(pointer, layout, size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure(operation: impl FnOnce()) -> (usize, usize) {
    ALLOCATIONS.set(Some((0, 0)));
    operation();
    ALLOCATIONS.replace(None).unwrap()
}

struct DropProbe(Rc<Cell<usize>>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

use tsonic_rust_js::timers;
use tsonic_rust_runtime::{Callable, TsonicError, TsonicResult};

struct NativeTimer {
    _callback: Rc<dyn Fn() -> TsonicResult<()>>,
    _delay: Duration,
    _due: Instant,
    _interval: bool,
}

fn callbacks(count: usize) -> Vec<Callable<(), TsonicResult<()>>> {
    (0..count)
        .map(|value| {
            Callable::new(move |()| {
                black_box(value);
                Ok(())
            })
        })
        .collect()
}

#[test]
fn retained_timer_allocations_match_one_native_callback_owner() {
    let mut native = BTreeMap::new();
    for count in [0, 1, 64, 1024] {
        assert!(!timers::has_timers());
        let actual_callbacks = callbacks(count);
        let expected_callbacks = callbacks(count);
        let mut handles = Vec::with_capacity(count);
        let delay = Duration::from_secs(60);
        let actual = measure(|| {
            for callback in actual_callbacks {
                handles.push(timers::set_timeout_callable(callback, 60_000.0));
            }
        });
        let expected = measure(|| {
            for (index, callback) in expected_callbacks.into_iter().enumerate() {
                native.insert(
                    index as u64,
                    NativeTimer {
                        _callback: Rc::new(move || callback.call(())),
                        _delay: delay,
                        _due: Instant::now() + delay,
                        _interval: false,
                    },
                );
            }
        });
        black_box(&native);
        for handle in handles {
            timers::clear_timeout(handle);
        }
        for index in 0..count {
            native.remove(&(index as u64));
        }
        assert!(!timers::has_timers());
        assert_eq!(actual, expected, "registrations={count}");
    }
}

#[test]
fn cancellation_releases_the_callback_without_invoking_it() {
    let drops = Rc::new(Cell::new(0));
    let probe = DropProbe(Rc::clone(&drops));
    let timer = timers::set_timeout_callable(
        Callable::new(move |()| -> TsonicResult<()> {
            black_box(&probe);
            panic!("cancelled timer executed");
        }),
        0.0,
    );
    assert_eq!(drops.get(), 0);
    timers::clear_timeout(timer);
    assert_eq!(drops.get(), 1);
    assert!(!timers::poll_timers().unwrap());
}

#[test]
fn shared_callbacks_can_schedule_and_poll_without_a_borrowed_registry() {
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let probe = DropProbe(Rc::clone(&drops));
    let callback_calls = Rc::clone(&calls);
    timers::set_timeout_callable(
        Callable::new(move |()| {
            black_box(&probe);
            callback_calls.set(callback_calls.get() + 1);
            let nested_calls = Rc::clone(&callback_calls);
            timers::set_timeout_callable(
                Callable::new(move |()| {
                    nested_calls.set(nested_calls.get() + 1);
                    Ok::<(), TsonicError>(())
                }),
                0.0,
            );
            assert!(timers::poll_timers()?);
            Ok::<(), TsonicError>(())
        }),
        0.0,
    );
    timers::run_timers().unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(drops.get(), 1);
    assert!(!timers::has_timers());
}

#[test]
fn intervals_retain_state_until_self_cancellation_and_release_after_dispatch() {
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let handle = Rc::new(Cell::new(0_u64));
    let probe = DropProbe(Rc::clone(&drops));
    let callback_calls = Rc::clone(&calls);
    let callback_handle = Rc::clone(&handle);
    handle.set(timers::set_interval_callable(
        Callable::new(move |()| {
            black_box(&probe);
            callback_calls.set(callback_calls.get() + 1);
            if callback_calls.get() == 2 {
                timers::clear_interval(callback_handle.get());
            }
            Ok::<(), TsonicError>(())
        }),
        1.0,
    ));
    timers::run_timers().unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(drops.get(), 1);
    assert!(!timers::has_timers());
}

#[test]
fn fallible_interval_retains_its_owner_until_cancelled() {
    let drops = Rc::new(Cell::new(0));
    let probe = DropProbe(Rc::clone(&drops));
    let timer = timers::set_interval_callable(
        Callable::new(move |()| {
            black_box(&probe);
            Err::<(), _>(std::io::Error::other("expected timer error"))
        }),
        1.0,
    );
    assert!(timers::run_timers()
        .unwrap_err()
        .to_string()
        .contains("expected timer error"));
    assert_eq!(drops.get(), 0);
    timers::clear_interval(timer);
    assert_eq!(drops.get(), 1);
    assert!(!timers::has_timers());
}
