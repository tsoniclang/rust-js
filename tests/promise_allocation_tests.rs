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

const fn counts(
    allocations: usize,
    deallocations: usize,
    reallocations: usize,
    bytes: usize,
) -> Counts {
    Counts {
        allocations,
        deallocations,
        reallocations,
        bytes,
    }
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

fn assert_row(row: &str, measured: Counts, expected: Counts) {
    record(row, measured);
    assert!(
        measured.allocations == expected.allocations,
        "{row}: {} allocations, expected {}",
        measured.allocations,
        expected.allocations
    );
    assert!(
        measured.deallocations == expected.deallocations,
        "{row}: {} deallocations, expected {}",
        measured.deallocations,
        expected.deallocations
    );
    assert!(
        measured.reallocations == expected.reallocations,
        "{row}: {} reallocations, expected {}",
        measured.reallocations,
        expected.reallocations
    );
    assert!(
        measured.bytes == expected.bytes,
        "{row}: {} requested bytes, expected {}",
        measured.bytes,
        expected.bytes
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

const COMPLETION: Counts = counts(1, 0, 0, 72);

#[test]
fn settled_constructors_allocate_only_the_completion() {
    let (promise, resolved) = measured(|| JsPromise::resolved(7_i32));
    assert_row("resolved(i32)", resolved, COMPLETION);
    drop(promise);

    let (promise, from_ok) = measured(|| JsPromise::from_result(Ok::<i32, JsValue>(7)));
    assert_row("from_result(Ok)", from_ok, COMPLETION);
    drop(promise);

    let reason = JsValue::Bool(false);
    let (promise, rejected) = measured(|| JsPromise::<i32>::rejected(reason));
    assert_row("rejected(JsValue)", rejected, COMPLETION);
    drop(promise);

    let reason = JsValue::Bool(true);
    let (promise, from_err) = measured(|| JsPromise::<i32>::from_result(Err(reason)));
    assert_row("from_result(Err)", from_err, COMPLETION);
    drop(promise);
}

#[test]
fn owned_string_and_retained_error_reasons_are_admitted_without_copies() {
    let text = JsValue::String(String::from("an owned reason"));
    let (promise, string) = measured(|| JsPromise::<i32>::rejected(text));
    assert_row("rejected(owned String reason)", string, COMPLETION);
    drop(promise);

    let error = JsValue::from_error(JsError::error("a retained error"));
    let (promise, retained) = measured(|| JsPromise::<i32>::rejected(error));
    assert_row("rejected(retained Error reason)", retained, COMPLETION);
    drop(promise);
}

#[test]
fn eager_factories_allocate_the_completion_and_one_task() {
    block_on(async {
        warm_owner().await;

        let body = async { 3 };
        println!(
            "promise-allocation row=eager factory, ready body body_bytes={}",
            std::mem::size_of_val(&body)
        );
        let (promise, ready_body) =
            measured(move || JsPromise::from_infallible_factory(move || body));
        assert_row(
            "eager factory, ready body",
            ready_body,
            counts(2, 1, 0, 152),
        );
        drop(promise);

        let reason = JsValue::Bool(false);
        let (promise, rejecting) = measured(|| {
            JsPromise::<i32>::from_fallible_factory(move || async move { Err(reason) })
        });
        assert_row(
            "eager factory, rejecting body",
            rejecting,
            counts(2, 1, 0, 200),
        );
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
        assert_row(
            "eager factory, pending, one awaiter",
            pending,
            counts(2, 0, 0, 168),
        );
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

fn padded_factory<const PADDING: usize>() -> (Counts, usize) {
    let body = async {
        let padding = black_box([0_u8; PADDING]);
        ready(()).await;
        black_box(&padding);
        1_u8
    };
    let size = std::mem::size_of_val(&body);
    let (promise, counts) = measured(move || JsPromise::from_infallible_factory(move || body));
    drop(promise);
    (counts, size)
}

fn padded_rejecting_factory<const PADDING: usize>() -> (Counts, usize) {
    let body = async {
        let padding = black_box([0_u8; PADDING]);
        ready(()).await;
        black_box(&padding);
        Err::<u8, JsValue>(JsValue::Bool(false))
    };
    let size = std::mem::size_of_val(&body);
    let (promise, counts) = measured(move || JsPromise::from_fallible_factory(move || body));
    drop(promise);
    (counts, size)
}

macro_rules! padding_scan {
    ($($padding:literal),* $(,)?) => {
        [$((padded_factory::<$padding> as fn() -> (Counts, usize), $padding)),*]
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
                let (counts, body_size) = measure();
                (*padding, counts, body_size)
            })
            .collect();
        for (padding, counts, body_size) in &measured {
            println!(
                "promise-allocation row=padded factory padding={padding} body_bytes={body_size} allocations={} deallocations={} reallocations={} bytes={}",
                counts.allocations, counts.deallocations, counts.reallocations, counts.bytes
            );
        }
        let boundary = measured
            .iter()
            .position(|(_, counts, _)| counts.allocations == 3)
            .expect("the scan crosses the 2,048-byte task threshold");
        assert!(boundary > 0, "the scan starts below the threshold");
        let (below_padding, below, below_size) = measured[boundary - 1];
        let (above_padding, above, above_size) = measured[boundary];
        println!(
            "promise-allocation row=threshold below_padding={below_padding} below_body_bytes={below_size} below_allocations={} above_padding={above_padding} above_body_bytes={above_size} above_allocations={}",
            below.allocations, above.allocations
        );
        for (padding, counts, body_size) in &measured {
            let boxed = counts.allocations == 3;
            assert!(
                counts.allocations == 2 || boxed,
                "padding {padding}: {} allocations",
                counts.allocations
            );
            assert!(
                boxed == (*padding >= above_padding),
                "padding {padding}: the box appears only at or above the threshold"
            );
            assert!(
                counts.deallocations == counts.allocations - 1,
                "padding {padding}: a ready body releases everything but the completion"
            );
            assert!(
                counts.reallocations == 0,
                "padding {padding}: {} reallocations",
                counts.reallocations
            );
            assert!(
                counts.bytes > *body_size,
                "padding {padding}: {} requested bytes hold a {body_size}-byte body",
                counts.bytes
            );
        }
    })
    .unwrap();
}

#[test]
fn a_rejecting_body_on_a_large_frame_adds_only_the_task_box() {
    block_on(async {
        warm_owner().await;
        let (small, small_size) = padded_rejecting_factory::<64>();
        println!("promise-allocation row=rejecting small frame body_bytes={small_size}");
        assert_row(
            "eager factory, rejecting body, small frame",
            small,
            counts(2, 1, 0, 216),
        );
        let (large, large_size) = padded_rejecting_factory::<2064>();
        println!("promise-allocation row=rejecting large frame body_bytes={large_size}");
        assert_row(
            "eager factory, rejecting body, large frame",
            large,
            counts(3, 2, 0, 2232),
        );
    })
    .unwrap();
}

#[test]
fn a_second_waiter_adds_one_out_of_line_allocation_and_later_waiters_follow_slab_growth() {
    let resolver = Rc::new(RefCell::new(None::<PromiseResolve<i32>>));
    let captured = Rc::clone(&resolver);
    let promise = JsPromise::create(Callable::new(
        move |(resolve, _): (PromiseResolve<i32>, PromiseReject)| {
            captured.replace(Some(resolve));
            Ok(())
        },
    ));
    let expected = [
        counts(0, 0, 0, 0),
        counts(1, 0, 0, 152),
        counts(0, 0, 0, 0),
        counts(0, 0, 0, 0),
        counts(1, 0, 0, 128),
        counts(0, 0, 0, 0),
        counts(0, 0, 0, 0),
        counts(0, 0, 0, 0),
        counts(0, 0, 1, 256),
    ];
    let mut waiting: Vec<_> = expected
        .iter()
        .map(|_| Box::pin(promise.await_result()))
        .collect();
    let mut context = Context::from_waker(Waker::noop());
    for (index, (future, expected)) in waiting.iter_mut().zip(expected).enumerate() {
        let row = format!("waiter {}", index + 1);
        let ((), waiter) = measured(|| assert!(future.as_mut().poll(&mut context).is_pending()));
        assert_row(&row, waiter, expected);
    }
    resolver
        .borrow_mut()
        .take()
        .unwrap()
        .call((PromiseResolution::Value(13),))
        .unwrap();
    for future in &mut waiting {
        assert!(matches!(
            future.as_mut().poll(&mut context),
            Poll::Ready(Ok(13))
        ));
    }
}

#[test]
fn then_on_a_settled_source_allocates_one_state_and_one_task() {
    block_on(async {
        warm_owner().await;
        let source = JsPromise::resolved(4);
        let callback = Callable::new(|(value,): (i32,)| Ok(value + 1));
        let (reaction, then) = measured(|| source.then(callback, None));
        assert_row("then on a settled source", then, counts(2, 0, 0, 248));
        assert_eq!(reaction.into_result().await.unwrap(), 5);
    })
    .unwrap();
}

async fn adoption_chain_counts(length: usize) -> Counts {
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
    for (resolve, source) in resolvers.iter().zip(&adopted) {
        resolve
            .call((PromiseResolution::Promise(source.clone()),))
            .unwrap();
    }
    yield_once().await;
    let counts = end();
    drop(adopted);
    let last = promises.pop().unwrap();
    assert_eq!(last.into_result().await.unwrap(), 21);
    counts
}

#[test]
fn adoption_chains_allocate_a_fixed_cost_per_link() {
    block_on(async {
        warm_owner().await;
        let mut per_link = None;
        for length in [1, 4, 16] {
            let chain = adoption_chain_counts(length).await;
            record(&format!("adoption chain length={length}"), chain);
            let link = counts(
                chain.allocations / length,
                chain.deallocations / length,
                chain.reallocations / length,
                chain.bytes / length,
            );
            let scaled = counts(
                link.allocations * length,
                link.deallocations * length,
                link.reallocations * length,
                link.bytes * length,
            );
            assert_row(
                &format!("adoption chain length={length} as links"),
                chain,
                scaled,
            );
            assert!(
                link.allocations <= 1,
                "k = {} allocations per link",
                link.allocations
            );
            assert!(
                *per_link.get_or_insert(link) == link,
                "the per-link cost is independent of the chain length"
            );
            assert_row("adoption link", link, counts(1, 1, 0, 152));
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
        let ((), observed) = measured(|| promise.observe_discarded());
        assert_row("observe_discarded, fulfilled", observed, counts(0, 1, 0, 0));
    })
    .unwrap();
}

fn observed_discard(reason: JsValue) -> Counts {
    let (observed, status) = block_on_with_contexts(
        async move {
            warm_owner().await;
            let promise = JsPromise::<i32>::rejected(reason);
            let ((), observed) = measured(|| promise.observe_discarded());
            observed
        },
        no_contexts(),
    )
    .unwrap();
    assert!(status.exit_requested());
    observed.expect("the root completes before the report job runs")
}

#[test]
fn observing_a_discarded_rejection_admits_one_reporting_job() {
    assert_row(
        "observe_discarded, rejected (Bool reason)",
        observed_discard(JsValue::Bool(false)),
        counts(1, 1, 0, 120),
    );
}

#[test]
fn observing_a_unique_discarded_string_rejection_moves_the_reason() {
    assert_row(
        "observe_discarded, rejected (String reason, 15 bytes)",
        observed_discard(JsValue::String(String::from("an owned reason"))),
        counts(1, 1, 0, 120),
    );
}

#[test]
fn a_cold_owner_has_a_fixed_entry_cost() {
    let ((_, first), cold) = measured(|| run_entry_with_contexts(|| (), no_contexts()).unwrap());
    let ((_, second), repeated) =
        measured(|| run_entry_with_contexts(|| (), no_contexts()).unwrap());
    assert_row("cold owner entry", cold, counts(2, 2, 0, 136));
    assert_row("repeated owner entry", repeated, counts(2, 2, 0, 136));
    assert!(!first.exit_requested() && !second.exit_requested());
}

#[test]
fn the_completion_state_layout_is_the_inline_reason_candidate() {
    let (promise, completion) = measured(|| JsPromise::resolved(0_i32));
    drop(promise);
    let words = 2 * std::mem::size_of::<usize>();
    println!(
        "promise-allocation row=layout completion_block_bytes={} rc_header_bytes={words} refcell_state_bytes={} jsvalue_bytes={}",
        completion.bytes,
        completion.bytes - words,
        std::mem::size_of::<JsValue>()
    );
    assert_eq!(std::mem::size_of::<JsValue>(), 40);
    assert!(
        completion.bytes <= 72,
        "the inline-reason completion block is {} bytes; the registered row is 72",
        completion.bytes
    );
    assert_row("layout", completion, COMPLETION);
}
