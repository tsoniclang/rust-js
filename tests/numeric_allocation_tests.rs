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
