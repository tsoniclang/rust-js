use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::future::{poll_fn, ready, Future};
use std::hint::black_box;
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::event_loop::{block_on, block_on_with_contexts, run_entry_with_contexts};
use tsonic_rust_js::promise::{JsPromise, PromiseReject, PromiseResolution, PromiseResolve};
use tsonic_rust_js::JsValue;
use tsonic_rust_runtime::dispatch::DispatchEnd;
use tsonic_rust_runtime::{Callable, JsError, TsonicError};

struct CountingAllocator;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    allocations: usize,
    deallocations: usize,
    reallocations: usize,
    bytes: usize,
}

thread_local! {
    static TRACKED: Cell<Option<Counts>> = const { Cell::new(None) };
}

fn track(update: impl FnOnce(&mut Counts)) {
    let _ = TRACKED.try_with(|tracked| {
        if let Some(mut counts) = tracked.get() {
            update(&mut counts);
            tracked.set(Some(counts));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        track(|counts| {
            counts.allocations += 1;
            counts.bytes += layout.size();
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        track(|counts| counts.deallocations += 1);
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        track(|counts| {
            counts.reallocations += 1;
            counts.bytes += size;
        });
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn begin() {
    TRACKED.with(|tracked| tracked.set(Some(Counts::default())));
}

fn end() -> Counts {
    TRACKED.with(|tracked| tracked.replace(None).expect("an open measurement"))
}

fn measured<Output>(operation: impl FnOnce() -> Output) -> (Output, Counts) {
    begin();
    let output = operation();
    (output, end())
}

fn record(row: &str, counts: Counts) {
    println!(
        "promise-allocation row={row} allocations={} deallocations={} reallocations={} bytes={}",
        counts.allocations, counts.deallocations, counts.reallocations, counts.bytes
    );
}

fn no_contexts() -> DispatchEnd<TsonicError> {
    DispatchEnd::<TsonicError>::new()
}

async fn yield_once() {
    let mut yielded = false;
    poll_fn(|context| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

async fn warm_owner() {
    for _ in 0..64 {
        drop(JsPromise::resolved(1).then(Callable::new(|(value,)| Ok(value)), None));
    }
    yield_once().await;
    for _ in 0..64 {
        drop(JsPromise::from_infallible_factory(|| async { 1 }));
    }
}

#[test]
fn settled_constructors_allocate_only_the_completion() {
    let (promise, resolved) = measured(|| JsPromise::resolved(7_i32));
    record("resolved(i32)", resolved);
    assert_eq!(resolved.allocations, 1);
    assert_eq!(resolved.reallocations, 0);
    assert!(
        resolved.bytes <= 72,
        "the inline-reason completion block is {} bytes; the registered row is 72",
        resolved.bytes
    );
    drop(promise);

    let (promise, from_ok) = measured(|| JsPromise::from_result(Ok::<i32, JsValue>(7)));
    record("from_result(Ok)", from_ok);
    assert_eq!(from_ok.allocations, 1);
    drop(promise);

    let reason = JsValue::Bool(false);
    let (promise, rejected) = measured(|| JsPromise::<i32>::rejected(reason));
    record("rejected(JsValue)", rejected);
    assert_eq!(rejected.allocations, 1);
    assert_eq!(rejected.bytes, resolved.bytes);
    drop(promise);

    let reason = JsValue::Bool(true);
    let (promise, from_err) = measured(|| JsPromise::<i32>::from_result(Err(reason)));
    record("from_result(Err)", from_err);
    assert_eq!(from_err.allocations, 1);
    drop(promise);
}

#[test]
fn owned_string_and_retained_error_reasons_are_admitted_without_copies() {
    let text = JsValue::String(String::from("an owned reason"));
    let (promise, string) = measured(|| JsPromise::<i32>::rejected(text));
    record("rejected(owned String reason)", string);
    assert_eq!(string.allocations, 1, "only the completion block");
    drop(promise);

    let error = JsValue::from_error(JsError::error("a retained error"));
    let (promise, retained) = measured(|| JsPromise::<i32>::rejected(error));
    record("rejected(retained Error reason)", retained);
    assert_eq!(retained.allocations, 1, "only the completion block");
    drop(promise);
}

#[test]
fn eager_factories_allocate_the_completion_and_one_task() {
    block_on(async {
        warm_owner().await;

        let (promise, ready_body) = measured(|| JsPromise::from_infallible_factory(|| async { 3 }));
        record("eager factory, ready body", ready_body);
        assert!(ready_body.allocations <= 2);
        assert_eq!(ready_body.reallocations, 0);
        drop(promise);

        let reason = JsValue::Bool(false);
        let (promise, rejecting) = measured(|| {
            JsPromise::<i32>::from_fallible_factory(move || async move { Err(reason) })
        });
        record("eager factory, rejecting body", rejecting);
        assert_eq!(rejecting.allocations, 2, "the completion and the task only");
        drop(promise);

        let gate = Rc::new(Cell::new(false));
        let source = Rc::new(RefCell::new(None::<Waker>));
        let body_gate = Rc::clone(&gate);
        let body_source = Rc::clone(&source);
        begin();
        let promise = JsPromise::from_infallible_factory(move || {
            poll_fn(move |context| {
                if body_gate.get() {
                    Poll::Ready(5)
                } else {
                    body_source.replace(Some(context.waker().clone()));
                    Poll::Pending
                }
            })
        });
        let mut awaiting = pin!(promise.await_result());
        assert!(awaiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending());
        let pending = end();
        record("eager factory, pending, one awaiter", pending);
        assert!(pending.allocations <= 2, "the first waiter is inline");
        gate.set(true);
        source.borrow_mut().take().unwrap().wake();
        yield_once().await;
        assert!(matches!(
            awaiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(5))
        ));
    })
    .unwrap();
}

fn padded_factory_allocations<const PADDING: usize>() -> (usize, usize) {
    let body = async {
        let padding = black_box([0_u8; PADDING]);
        ready(()).await;
        black_box(&padding);
        1_u8
    };
    let size = std::mem::size_of_val(&body);
    let (promise, counts) = measured(move || JsPromise::from_infallible_factory(move || body));
    drop(promise);
    (counts.allocations, size)
}

macro_rules! padding_scan {
    ($($padding:literal),* $(,)?) => {
        [$((padded_factory_allocations::<$padding> as fn() -> (usize, usize), $padding)),*]
    };
}

#[test]
fn the_task_box_threshold_adds_exactly_one_allocation() {
    block_on(async {
        warm_owner().await;
        let scan = padding_scan![
            1920, 1928, 1936, 1944, 1952, 1960, 1968, 1976, 1984, 1992, 2000, 2008, 2016, 2024,
            2032, 2040, 2048, 2056, 2064,
        ];
        let measured: Vec<_> = scan
            .iter()
            .map(|(measure, padding)| {
                let (allocations, body_size) = measure();
                (*padding, allocations, body_size)
            })
            .collect();
        for (padding, allocations, body_size) in &measured {
            println!(
                "promise-allocation row=padded factory padding={padding} body_bytes={body_size} allocations={allocations}"
            );
        }
        let boundary = measured
            .iter()
            .position(|(_, allocations, _)| *allocations == 3)
            .expect("the scan crosses the 2,048-byte task threshold");
        assert!(boundary > 0, "the scan starts below the threshold");
        let (below_padding, below, below_size) = measured[boundary - 1];
        let (above_padding, above, above_size) = measured[boundary];
        println!(
            "promise-allocation row=threshold below_padding={below_padding} below_body_bytes={below_size} below_allocations={below} above_padding={above_padding} above_body_bytes={above_size} above_allocations={above}"
        );
        assert!(below <= 2, "just below the threshold");
        assert!(above <= 3, "at or above the threshold");
        assert!(measured[..boundary].iter().all(|(_, allocations, _)| *allocations <= 2));
        assert!(measured[boundary..].iter().all(|(_, allocations, _)| *allocations == 3));
    })
    .unwrap();
}

#[test]
fn a_second_waiter_adds_one_out_of_line_allocation() {
    let resolver = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
    let captured = Rc::clone(&resolver);
    let promise = JsPromise::create(Callable::new(
        move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
            captured.replace(Some(resolve));
            Ok(())
        },
    ));
    let mut first = pin!(promise.await_result());
    let mut second = pin!(promise.await_result());
    let mut third = pin!(promise.await_result());
    let mut context = Context::from_waker(Waker::noop());
    let ((), first_waiter) = measured(|| assert!(first.as_mut().poll(&mut context).is_pending()));
    record("first waiter", first_waiter);
    assert_eq!(first_waiter.allocations, 0);
    let ((), second_waiter) = measured(|| assert!(second.as_mut().poll(&mut context).is_pending()));
    record("second waiter", second_waiter);
    assert_eq!(second_waiter.allocations, 1);
    let ((), third_waiter) = measured(|| assert!(third.as_mut().poll(&mut context).is_pending()));
    record("third waiter", third_waiter);
    assert_eq!(third_waiter.allocations, 0);
    resolver
        .borrow_mut()
        .take()
        .unwrap()
        .call((PromiseResolution::Value(13),))
        .unwrap();
    assert!(matches!(
        first.as_mut().poll(&mut context),
        Poll::Ready(Ok(13))
    ));
}

#[test]
fn then_on_a_settled_source_allocates_one_state_and_one_task() {
    block_on(async {
        warm_owner().await;
        let source = JsPromise::resolved(4);
        let callback = Callable::new(|(value,): (i32,)| Ok(value + 1));
        let (reaction, then) = measured(|| source.then(callback, None));
        record("then on a settled source", then);
        assert_eq!(then.allocations, 2);
        assert_eq!(reaction.into_result().await.unwrap(), 5);
    })
    .unwrap();
}

async fn adoption_chain_allocations(length: usize) -> usize {
    let mut resolvers = Vec::with_capacity(length);
    let mut promises = Vec::with_capacity(length + 1);
    promises.push(JsPromise::resolved(21));
    for _ in 0..length {
        let slot = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
        let captured = Rc::clone(&slot);
        promises.push(JsPromise::create(Callable::new(
            move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
                captured.replace(Some(resolve));
                Ok(())
            },
        )));
        resolvers.push(slot.borrow_mut().take().unwrap());
    }
    let adopted: Vec<_> = promises[..length].to_vec();
    begin();
    for (resolve, source) in resolvers.iter().zip(adopted) {
        resolve.call((PromiseResolution::Promise(source),)).unwrap();
    }
    yield_once().await;
    let counts = end();
    let last = promises.pop().unwrap();
    assert_eq!(last.into_result().await.unwrap(), 21);
    counts.allocations
}

#[test]
fn adoption_chains_allocate_a_fixed_cost_per_link() {
    block_on(async {
        warm_owner().await;
        let mut per_link = None;
        for length in [1, 4, 16] {
            let allocations = adoption_chain_allocations(length).await;
            println!(
                "promise-allocation row=adoption chain length={length} allocations={allocations}"
            );
            let k = allocations / length;
            assert_eq!(allocations, k * length, "independent of the chain length");
            assert!(k <= 1);
            assert_eq!(*per_link.get_or_insert(k), k);
        }
    })
    .unwrap();
}

#[derive(Debug)]
struct CountedClone(Rc<Cell<usize>>);

impl Clone for CountedClone {
    fn clone(&self) -> Self {
        self.0.set(self.0.get() + 1);
        Self(Rc::clone(&self.0))
    }
}

#[test]
fn unique_ready_and_pending_consumes_copy_no_payload() {
    block_on(async {
        let clones = Rc::new(Cell::new(0));
        let ready = JsPromise::resolved(CountedClone(Rc::clone(&clones)));
        let _ = ready.into_result().await.unwrap();
        let body_clones = Rc::clone(&clones);
        let pending = JsPromise::from_infallible_factory(move || async move {
            yield_once().await;
            CountedClone(body_clones)
        });
        let _ = pending.into_result().await.unwrap();
        println!(
            "promise-allocation row=unique consume payload_clones={}",
            clones.get()
        );
        assert_eq!(clones.get(), 0);
    })
    .unwrap();
}

#[test]
fn observing_a_discarded_fulfilled_promise_allocates_nothing() {
    block_on(async {
        warm_owner().await;
        let promise = JsPromise::resolved(1);
        let ((), counts) = measured(|| promise.observe_discarded());
        record("observe_discarded, fulfilled", counts);
        assert_eq!(counts.allocations, 0);
        assert_eq!(counts.deallocations, 1, "the dropped completion only");
    })
    .unwrap();
}

#[test]
fn observing_a_discarded_rejection_admits_one_reporting_job() {
    let (counts, status) = block_on_with_contexts(
        async {
            warm_owner().await;
            let promise = JsPromise::<i32>::rejected(JsValue::Bool(false));
            let ((), counts) = measured(|| promise.observe_discarded());
            counts
        },
        no_contexts(),
    )
    .unwrap();
    let counts = counts.expect("the root completes before the report job runs");
    record("observe_discarded, rejected", counts);
    assert!(counts.allocations <= 1, "at most one reporting job");
    assert!(status.exit_requested());
}

#[test]
fn a_cold_owner_has_a_fixed_entry_cost() {
    let ((_, first), cold) = measured(|| run_entry_with_contexts(|| (), no_contexts()).unwrap());
    let ((_, second), repeated) =
        measured(|| run_entry_with_contexts(|| (), no_contexts()).unwrap());
    record("cold owner entry", cold);
    record("repeated owner entry", repeated);
    assert!(!first.exit_requested() && !second.exit_requested());
    assert_eq!(cold.allocations, repeated.allocations);
}

#[test]
fn the_completion_state_layout_is_the_inline_reason_candidate() {
    let (promise, counts) = measured(|| JsPromise::resolved(0_i32));
    drop(promise);
    let words = 2 * std::mem::size_of::<usize>();
    println!(
        "promise-allocation row=layout completion_block_bytes={} rc_header_bytes={words} refcell_state_bytes={} jsvalue_bytes={}",
        counts.bytes,
        counts.bytes - words,
        std::mem::size_of::<JsValue>()
    );
    assert_eq!(std::mem::size_of::<JsValue>(), 40);
    assert!(counts.bytes <= 72);
}
