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
fn native_record_admission_reuses_one_backing_with_zero_additional_allocations() {
    use tsonic_rust_js::{equality::JsStrictEqual, JsValue};
    use tsonic_rust_runtime::Record;
    let original =
        Record::from_entries([(String::from("value"), JsValue::UnsignedInteger(u64::MAX))]);
    let distinct =
        Record::from_entries([(String::from("value"), JsValue::UnsignedInteger(u64::MAX))]);
    let identity = original.storage_identity_key();
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let value = JsValue::from(original.clone());
    let alias = JsValue::from(original.clone());
    let other = JsValue::from(distinct);
    for _iteration in 0..10_000 {
        assert!(black_box(&value).strict_equal(black_box(&alias)));
        assert!(!black_box(&value).strict_equal(black_box(&other)));
        assert_eq!(value.reference_identity_key(), Some(identity));
        assert_eq!(
            value.as_record().unwrap().get("value"),
            JsValue::UnsignedInteger(u64::MAX)
        );
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    if usize::BITS == 64 {
        assert_eq!(std::mem::size_of::<JsValue>(), 40);
    }
}

#[test]
fn native_shared_identity_admission_preserves_the_original_owner_without_allocating() {
    use std::rc::Rc;
    use tsonic_rust_js::{equality::JsStrictEqual, JsValue};
    use tsonic_rust_runtime::{EmptyObject, ObjectHandle};
    struct Dropped(Rc<Cell<usize>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let object = ObjectHandle::new(Dropped(drops.clone()));
    let empty = EmptyObject::new();
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let left = JsValue::from_shared_identity(object.clone().into_shared());
    let right = JsValue::from_shared_identity(object.clone().into_shared());
    let empty_value = JsValue::from(empty.clone());
    let empty_alias = JsValue::from(empty);
    for _ in 0..1000 {
        assert!(black_box(&left).strict_equal(black_box(&right)));
        assert!(black_box(&empty_value).strict_equal(black_box(&empty_alias)));
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    if usize::BITS == 64 {
        assert_eq!(std::mem::size_of::<JsValue>(), 40);
    }
    assert_eq!(left.type_of(), "object");
    assert!(tsonic_rust_js::value::closed_value_string(&left).is_err());
    if let JsValue::Closed(value) = &left {
        assert!(value.project_json().is_err());
    } else {
        panic!("native identity must be closed");
    }
    drop(object);
    drop(left);
    assert_eq!(drops.get(), 0);
    drop(right);
    assert_eq!(drops.get(), 1);
}

#[test]
fn native_passive_erasure_allocates_one_owner_and_never_inspects_its_payload() {
    use std::rc::Rc;
    use tsonic_rust_js::{equality::JsStrictEqual, JsValue};
    struct Dropped(Rc<Cell<usize>>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let payload = Dropped(drops.clone());
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let value = JsValue::from_closed(payload);
    let alias = value.clone();
    let equals = value.strict_equal(&alias);
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 1);
    assert!(equals);
    assert_eq!(value.inspect(), "[Native value]");
    assert!(tsonic_rust_js::value::closed_value_string(&value).is_err());
    if let JsValue::Closed(value) = &value {
        assert!(value.project_json().is_err());
    } else {
        panic!("native payload must be closed");
    }
    drop(value);
    assert_eq!(drops.get(), 0);
    drop(alias);
    assert_eq!(drops.get(), 1);
}

#[test]
fn closed_string_conversion_allocates_only_its_requested_result() {
    use tsonic_rust_js::{abi, JsArray, JsValue};
    let value = JsValue::String("a native string with enough bytes".to_owned());
    let values = JsValue::array(JsArray::from_dense(vec![
        JsValue::Integer(42),
        JsValue::Null,
    ]));
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let result = abi::closed_value_string(black_box(&value)).unwrap();
    let scalar_allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(scalar_allocations, 1);
    assert_eq!(result, "a native string with enough bytes");
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let result = abi::closed_value_string(black_box(&values)).unwrap();
    let array_allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(array_allocations, 1);
    assert_eq!(result, "42,");
}

#[test]
fn optional_primitive_conversion_avoids_boxing_and_intermediate_strings() {
    use tsonic_rust_js::{abi, string::JsToString};
    let optional_text = Some(" 42 ".to_owned());
    let integer = Some(9_007_199_254_740_993_i64);
    let mut buffer = String::with_capacity(32);
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for _ in 0..1000 {
        black_box(abi::number_from_value(black_box(&optional_text)));
        black_box(abi::number_from_value(black_box(&integer)));
        black_box(abi::number_from_value(black_box(&None::<u64>)));
        buffer.clear();
        black_box(&integer).write_js_string(&mut buffer);
        black_box(&buffer);
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert_eq!(buffer, "9007199254740993");
}

#[test]
fn closed_native_primitives_box_without_allocating() {
    use tsonic_rust_js::{equality::JsHash, JsValue};
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for _ in 0..1000 {
        macro_rules! check {
            ($($value:expr),+ $(,)?) => {$({
                let boxed = JsValue::from(black_box($value));
                black_box(boxed.js_hash());
                black_box(boxed.numeric_ref());
                black_box(boxed.type_of());
                black_box(boxed);
            })+};
        }
        check!(
            i8::MIN,
            u8::MAX,
            i16::MIN,
            u16::MAX,
            i32::MIN,
            u32::MAX,
            i64::MIN,
            u64::MAX,
            isize::MIN,
            usize::MAX,
            0.1_f32,
            0.1_f64
        );
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
}

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
