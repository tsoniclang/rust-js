use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

use tsonic_rust_js::abi::SourceNumeric;
use tsonic_rust_js::number::JsNumberValue;
use tsonic_rust_runtime::BigInt;

struct CountingAllocator;

thread_local! {
    static TRACKED_ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
    static TRACKED_BYTES: Cell<Option<usize>> = const { Cell::new(None) };
    static TRACKED_ALIGNMENT: Cell<Option<usize>> = const { Cell::new(None) };
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACKED_ALLOCATIONS.with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + 1));
            }
        });
        TRACKED_BYTES.with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + layout.size()));
            }
        });
        TRACKED_ALIGNMENT.with(|alignment| {
            if let Some(value) = alignment.get() {
                alignment.set(Some(value.max(layout.align())));
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

fn measured_allocation<Output>(
    operation: impl FnOnce() -> Output,
) -> (Output, usize, usize, usize) {
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    TRACKED_BYTES.with(|count| count.set(Some(0)));
    TRACKED_ALIGNMENT.with(|alignment| alignment.set(Some(0)));
    let output = operation();
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    let bytes = TRACKED_BYTES.with(|count| count.replace(None).unwrap());
    let alignment = TRACKED_ALIGNMENT.with(|value| value.replace(None).unwrap());
    (output, allocations, bytes, alignment)
}

#[test]
fn broad_empty_object_freeze_keeps_aliases_and_native_zero_allocation_cost() {
    use tsonic_rust_js::{
        equality::{JsHash, JsSameValue, JsSameValueZero, JsStrictEqual},
        JsValue,
    };
    use tsonic_rust_runtime::EmptyObject;
    fn retain(value: JsValue) -> JsValue {
        value
    }
    let original = EmptyObject::new();
    let other = EmptyObject::new();
    let first = JsValue::from(original.clone());
    let (result, allocations, bytes, _) = measured_allocation(|| {
        let alias = retain(first.clone());
        let initially_unfrozen = !alias.object_state_is_frozen();
        let frozen = first.freeze_object_state();
        let observations = (
            initially_unfrozen,
            first.strict_equal(&alias),
            frozen.strict_equal(&alias),
            frozen.same_value(&alias),
            frozen.same_value_zero(&alias),
            frozen.js_hash() == alias.js_hash(),
            first.object_state_is_frozen(),
            alias.object_state_is_frozen(),
            frozen.object_state_is_frozen(),
        );
        (alias, frozen, observations)
    });
    assert_eq!(allocations, 0);
    assert_eq!(bytes, 0);
    let (alias, frozen, observations) = result;
    assert_eq!(
        observations,
        (true, true, true, true, true, true, true, true, true)
    );
    assert!(original.is_frozen());
    assert!(!other.is_frozen());
    assert!(first.strict_equal(&JsValue::from(original.clone())));
    assert!(!frozen.strict_equal(&JsValue::from(other.clone())));
    assert_eq!(
        first.reference_identity_key(),
        alias.reference_identity_key()
    );
    assert_eq!(
        first.reference_identity_key(),
        frozen.reference_identity_key()
    );
    assert_eq!(frozen.type_of(), "object");
    let weak = original.into_identity().downgrade();
    drop(first);
    assert!(weak.is_alive());
    drop(alias);
    assert!(weak.is_alive());
    drop(frozen);
    assert!(!weak.is_alive());
}

#[test]
fn broad_empty_object_construction_and_freeze_match_direct_native_cost_and_layout() {
    use tsonic_rust_js::{equality::JsStrictEqual, JsClosedValue, JsValue};
    use tsonic_rust_runtime::EmptyObject;
    let (direct, direct_count, direct_bytes, direct_alignment) = measured_allocation(|| {
        let first = EmptyObject::new();
        let alias = first.clone();
        let frozen = first.freeze();
        (first, alias, frozen)
    });
    let (broad, count, bytes, alignment) = measured_allocation(|| {
        let first = JsValue::from(EmptyObject::new());
        let alias = first.clone();
        let frozen = first.freeze_object_state();
        (first, alias, frozen)
    });
    assert_eq!(direct_count, 1);
    assert_eq!(count, direct_count);
    assert_eq!(bytes, direct_bytes);
    assert_eq!(alignment, direct_alignment);
    assert_eq!(direct.1, direct.2);
    assert!(broad.1.strict_equal(&broad.2));
    if usize::BITS == 64 {
        assert_eq!(std::mem::size_of::<JsClosedValue>(), 24);
        assert_eq!(std::mem::align_of::<JsClosedValue>(), 8);
        assert_eq!(std::mem::size_of::<JsValue>(), 40);
        assert_eq!(std::mem::align_of::<JsValue>(), 8);
    }
}

#[test]
fn native_payload_does_not_grow_closed_js_carriers_or_their_allocation() {
    use std::mem::{align_of, size_of};
    use std::rc::Rc;
    use tsonic_rust_js::{JsClosedValue, JsValue};
    use tsonic_rust_runtime::NativePayload;
    #[repr(align(64))]
    struct Aligned([u8; 64]);
    fn check<Payload: 'static>(create: impl Fn() -> Payload) {
        let direct = create();
        let closed = create();
        let (direct, direct_count, direct_bytes, direct_alignment) =
            measured_allocation(|| Rc::new(direct));
        let (closed, closed_count, closed_bytes, closed_alignment) =
            measured_allocation(|| JsValue::from_closed(closed));
        assert_eq!(direct_count, 1);
        assert_eq!(closed_count, 1);
        assert_eq!(closed_bytes, direct_bytes);
        assert_eq!(closed_alignment, direct_alignment);
        drop((direct, closed));
    }
    check(|| ());
    check(|| u64::MAX);
    check(|| u128::MAX);
    check(|| [7_u8; 256]);
    check(|| String::from("owned native string"));
    check(|| Aligned([7; 64]));
    assert_eq!(Aligned([7; 64]).0[0], 7);
    if usize::BITS == 64 {
        assert_eq!(size_of::<NativePayload>(), 16);
        assert_eq!(align_of::<NativePayload>(), 8);
        assert_eq!(size_of::<JsClosedValue>(), 24);
        assert_eq!(align_of::<JsClosedValue>(), 8);
        assert_eq!(size_of::<JsValue>(), 40);
        assert_eq!(align_of::<JsValue>(), 8);
    }
}

#[test]
fn native_passive_aliases_and_queries_preserve_exact_clone_and_drop_costs() {
    use std::rc::Rc;
    use tsonic_rust_js::{
        equality::{JsHash, JsSameValue, JsSameValueZero, JsStrictEqual},
        JsValue,
    };
    struct Probe {
        clones: Rc<Cell<usize>>,
        drops: Rc<Cell<usize>>,
        value: u64,
    }
    impl Clone for Probe {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                clones: self.clones.clone(),
                drops: self.drops.clone(),
                value: self.value,
            }
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    let clones = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let value = JsValue::from_closed(Probe {
        clones: clones.clone(),
        drops: drops.clone(),
        value: u64::MAX,
    });
    let identity = value.reference_identity_key();
    let (alias, count, bytes, _) = measured_allocation(|| value.clone());
    assert_eq!(count, 0);
    assert_eq!(bytes, 0);
    assert_eq!(clones.get(), 0);
    assert_eq!(alias.reference_identity_key(), identity);
    let (observations, count, bytes, _) = measured_allocation(|| {
        (
            value.strict_equal(&alias),
            value.same_value(&alias),
            value.same_value_zero(&alias),
            value.js_hash() == alias.js_hash(),
            alias.native_value::<u64>(),
        )
    });
    assert_eq!(observations, (true, true, true, true, None));
    assert_eq!(count, 0);
    assert_eq!(bytes, 0);
    assert_eq!(clones.get(), 0);
    let (recovered, count, bytes, _) = measured_allocation(|| alias.native_value::<Probe>());
    assert_eq!(count, 0);
    assert_eq!(bytes, 0);
    assert_eq!(clones.get(), 1);
    let recovered = recovered.unwrap();
    assert_eq!(recovered.value, u64::MAX);
    drop(recovered);
    assert_eq!(drops.get(), 1);
    drop(value);
    assert_eq!(drops.get(), 1);
    drop(alias);
    assert_eq!(drops.get(), 2);
}

#[test]
fn exact_native_string_recovery_clones_only_the_requested_payload() {
    use tsonic_rust_js::JsValue;
    let original = String::from("one explicitly requested native string clone");
    let retained = JsValue::from_closed(original.clone());
    let (direct, direct_count, direct_bytes, direct_alignment) =
        measured_allocation(|| original.clone());
    let (recovered, count, bytes, alignment) =
        measured_allocation(|| retained.native_value::<String>().unwrap());
    assert_eq!(count, 1);
    assert_eq!(count, direct_count);
    assert_eq!(bytes, direct_bytes);
    assert_eq!(alignment, direct_alignment);
    assert_eq!(recovered, direct);
}

#[test]
fn generic_number_predicates_borrow_existing_numeric_storage_without_allocations() {
    use tsonic_rust_js::number::{self, NativeNumberPredicate};
    fn integer<Value: NativeNumberPredicate>(value: &Value) -> bool {
        number::is_integer(value)
    }
    let large = tsonic_rust_js::bigint::from_string("9007199254740993").unwrap();
    let union = tsonic_rust_js::number::JsNumeric::from_bigint(&large);
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    for _iteration in 0..10_000 {
        assert!(integer(black_box(&large)));
        assert!(integer(black_box(&union)));
        assert!(integer(black_box(&u64::MAX)));
        assert!(!integer(black_box(&1.5_f64)));
        assert!(!number::is_safe_integer(black_box(&large)));
        assert!(number::is_finite(black_box(&union)));
        assert!(!number::is_nan(black_box(&union)));
    }
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    assert_eq!(large.to_string(), "9007199254740993");
}

#[test]
fn native_record_json_borrows_scalar_keys_instead_of_snapshotting_every_string() {
    use tsonic_rust_js::{json, JsArray, JsValue};
    use tsonic_rust_runtime::Record;
    let record = Record::from_entries((0..1000).map(|index| {
        (
            format!("field_{index:04}_with_a_long_native_string_key_that_must_not_be_copied"),
            JsValue::Bool(true),
        )
    }));
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let requested_keys = black_box(&record).keys();
    let snapshot_allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(snapshot_allocations, 1001);
    let value = JsValue::from(record.clone());
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let rendered = json::stringify(black_box(&value)).unwrap().unwrap();
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    eprintln!("1000 native keys: requested snapshot allocations={snapshot_allocations}; default JSON allocations={allocations}");
    assert!(
        allocations <= 24,
        "default serialization unexpectedly copied native keys: {allocations}"
    );
    let nested = JsValue::from(Record::from_entries(requested_keys.iter().map(|key| {
        (
            key.clone(),
            JsValue::array(JsArray::from_dense(vec![JsValue::Bool(true)])),
        )
    })));
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let nested_rendered = json::stringify(black_box(&nested)).unwrap().unwrap();
    let nested_allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    eprintln!("1000 native keys with nested arrays: conservative default JSON allocations={nested_allocations}");
    for key in requested_keys {
        assert!(rendered.contains(&format!("\"{key}\":true")));
        assert!(nested_rendered.contains(&format!("\"{key}\":[true]")));
    }
    assert!(record.contains_key("field_0000_with_a_long_native_string_key_that_must_not_be_copied"));
}

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
fn native_shared_object_conversions_retain_aliases_without_additional_allocations() {
    use std::rc::Rc;
    use tsonic_rust_js::{equality::JsStrictEqual, JsValue};
    use tsonic_rust_runtime::{ObjectHandle, ObjectHandleState, ObjectRef, ObjectRefState};
    struct Payload {
        count: Cell<u64>,
        drops: Rc<Cell<usize>>,
    }
    impl Drop for Payload {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let handle = ObjectHandle::new(Payload {
        count: Cell::new(u64::MAX),
        drops: drops.clone(),
    });
    let reference = ObjectRef::new(Payload {
        count: Cell::new(u64::MAX),
        drops: drops.clone(),
    });
    TRACKED_ALLOCATIONS.with(|count| count.set(Some(0)));
    let left = JsValue::from(handle.clone());
    let right = JsValue::from(handle.clone());
    let shared = JsValue::from(reference.clone());
    let shared_alias = JsValue::from(reference.clone());
    let handle_projection = left.native_shared::<ObjectHandleState<Payload>>().unwrap();
    let reference_projection = shared.native_shared::<ObjectRefState<Payload>>().unwrap();
    assert!(Rc::ptr_eq(&handle.shared(), &handle_projection));
    assert!(Rc::ptr_eq(&reference.shared(), &reference_projection));
    let recovered = ObjectHandle::from_shared(handle_projection);
    let recovered_reference = ObjectRef::from_shared(reference_projection);
    handle.with_mut(|value| value.count.set(7));
    reference.with(|value| value.count.set(11));
    let allocations = TRACKED_ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(allocations, 0);
    for _iteration in 0..10_000 {
        assert!(black_box(&left).strict_equal(black_box(&right)));
        assert!(black_box(&shared).strict_equal(black_box(&shared_alias)));
        assert!(!black_box(&left).strict_equal(black_box(&shared)));
        assert_eq!(recovered.with(|value| value.count.get()), 7);
        assert_eq!(recovered_reference.with(|value| value.count.get()), 11);
    }
    drop(handle);
    drop(reference);
    drop(left);
    drop(right);
    drop(shared);
    drop(shared_alias);
    assert_eq!(drops.get(), 0);
    drop(recovered);
    assert_eq!(drops.get(), 1);
    drop(recovered_reference);
    assert_eq!(drops.get(), 2);
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
fn json_native_integer_formatting_matches_direct_native_output_cost() {
    use std::fmt::{Display, Write};
    use tsonic_rust_js::{json, JsValue};
    fn check<Value: Display>(value: Value, carrier: JsValue) {
        let (expected, native_count, native_bytes, native_alignment) = measured_allocation(|| {
            let mut result = String::new();
            write!(&mut result, "{value}").unwrap();
            result
        });
        let (actual, count, bytes, alignment) =
            measured_allocation(|| json::stringify(&carrier).unwrap().unwrap());
        assert_eq!(actual, expected);
        assert_eq!(count, native_count, "{expected}");
        assert_eq!(bytes, native_bytes, "{expected}");
        assert_eq!(alignment, native_alignment, "{expected}");
    }
    macro_rules! check_carrier {
        ($variant:ident, $($value:expr),+ $(,)?) => { $(
            let value = $value;
            check(value, JsValue::$variant(value));
        )+ };
    }
    check_carrier!(
        Integer,
        i64::MIN,
        i64::MAX,
        9_007_199_254_740_993_i64,
        0_i64
    );
    check_carrier!(UnsignedInteger, u64::MAX, 9_007_199_254_740_993_u64, 0_u64);
    check_carrier!(Int8, i8::MIN, i8::MAX, 0_i8);
    check_carrier!(Uint8, u8::MAX, 0_u8);
    check_carrier!(Int16, i16::MIN, i16::MAX, 0_i16);
    check_carrier!(Uint16, u16::MAX, 0_u16);
    check_carrier!(Int32, i32::MIN, i32::MAX, 0_i32);
    check_carrier!(Uint32, u32::MAX, 0_u32);
    check_carrier!(NativeInt, isize::MIN, isize::MAX, 0_isize);
    check_carrier!(NativeUint, usize::MAX, 0_usize);
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
            isize::MIN,
            usize::MAX,
            1.5_f32,
            1.5_f64
        );
        valid &= SourceNumeric::greater_than(&explicit_bigint, &black_box(9007199254740992_f64));
        valid &= SourceNumeric::strict_equal(&black_box(0_usize), &black_box(0_f64));
        valid &= SourceNumeric::less_than(&black_box(isize::MIN), &black_box(0_f64));
        valid &= SourceNumeric::greater_than(&black_box(usize::MAX), &black_box(0_f64));
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
