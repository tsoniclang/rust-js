use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

use tsonic_rust_js::abi::SourceNumeric;
use tsonic_rust_js::number::JsNumberValue;
use tsonic_rust_runtime::BigInt;

struct CountingAllocator;

thread_local! {
    static TRACKED_ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACKED_ALLOCATIONS.with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn typed_array_search_does_not_allocate_for_native_or_unrepresentable_queries() {
    use tsonic_rust_js::{Float64Array, Uint8Array};
    let bytes = Uint8Array::from_bytes(vec![0, 1, 255]);
    let doubles = Float64Array::from_vec(vec![9_007_199_254_740_992_f64, u64::MAX as f64]).unwrap();
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for _ in 0..1000 {
        black_box(bytes.includes_from_start(black_box(255_u8)));
        black_box(bytes.index_of_from_start(black_box(u64::MAX)));
        black_box(doubles.includes_from_start(black_box(9_007_199_254_740_993_u64)));
        black_box(doubles.index_of_from_start(black_box(u64::MAX)));
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
}

#[test]
fn numeric_array_copy_borrows_without_allocating_or_boxing_elements() {
    use tsonic_rust_js::{array::JsArray, Uint8Array};
    let values = JsArray::from_dense(vec![9_007_199_254_740_993_u64, u64::MAX]);
    let bytes = Uint8Array::new(2_usize).unwrap();
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for _ in 0..1000 {
        bytes.set_from_array(black_box(&values), 0_usize).unwrap();
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert_eq!(bytes.get_number(0_usize), Some(1));
    assert_eq!(bytes.get_number(1_usize), Some(255));
}

#[test]
fn numeric_formatting_allocates_only_the_result_string() {
    let formats: [fn() -> String; 5] = [
        || 1.25_f64.fixed_string(1),
        || 1.25_f64.exponential_string(Some(1)),
        || 1.25_f64.precision_string(2),
        || 9007199254740993_i64.fixed_string(100),
        || u128::MAX.precision_string(100),
    ];
    for format in formats {
        TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
        for _ in 0..1000 {
            black_box(format());
        }
        let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
        assert_eq!(allocations, 1000);
    }
}

#[test]
fn native_numeric_conversion_and_borrowed_comparison_do_not_allocate() {
    let explicit_bigint = BigInt::from_decimal_literal("9007199254740993");
    let enormous = BigInt::from_decimal_literal("340282366920938463463374607431768211456");
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let mut valid = true;
    for _ in 0..1000 {
        macro_rules! native {
            ($($value:expr),+ $(,)?) => {$(
                let value = black_box($value);
                valid &= SourceNumeric::to_number(&value) == value as f64;
                valid &= SourceNumeric::strict_equal(&value, &value);
            )+};
        }
        native!(
            i8::MIN,
            u8::MAX,
            i16::MIN,
            u16::MAX,
            i32::MIN,
            u32::MAX,
            i64::MIN,
            u64::MAX,
            i128::MIN,
            u128::MAX,
            1.5_f32,
            1.5_f64
        );
        valid &= SourceNumeric::greater_than(&explicit_bigint, &black_box(9007199254740992_f64));
        valid &= SourceNumeric::strict_equal(&explicit_bigint, &black_box(9007199254740993_u64));
        valid &= SourceNumeric::greater_than(&enormous, &black_box(u128::MAX));
        valid &= SourceNumeric::loose_equal(&enormous, &black_box(2_f64.powi(128)));
        valid &= tsonic_rust_js::bigint::as_int_n(256.0, &enormous).unwrap() == enormous;
        valid &= tsonic_rust_js::bigint::as_uint_n(129.0, &enormous).unwrap() == enormous;
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert!(valid);
    assert_eq!(allocations, 0);
}
