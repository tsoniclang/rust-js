#[path = "../../../rust-runtime/tests/helpers/allocation_counts.rs"]
mod allocation_counts;

use std::cell::Cell;
use std::hint::black_box;
use tsonic_rust_js::equality::{JsHash, JsSameValueZero, JsStrictEqual};
use tsonic_rust_js::{JsMap, JsObject, JsSet, JsString, JsValue};
use tsonic_rust_runtime::{BigInt, Callable};

fn sample_object() -> JsValue {
    JsValue::object(JsObject::from_pairs([("x", JsValue::Number(1.0))]))
}

fn sample_array() -> JsValue {
    JsValue::from(vec![
        JsValue::Number(1.0),
        JsValue::Utf16String(JsString::from_utf8("a")),
    ])
}

#[test]
fn structurally_identical_objects_are_not_equal() {
    let left = sample_object();
    let right = sample_object();
    assert!(!left.strict_equal(&right));
    assert!(!left.same_value_zero(&right));
    assert_ne!(left, right);
}

#[test]
fn aliased_object_handles_are_identical() {
    let original = sample_object();
    let alias = original.clone();
    assert!(original.strict_equal(&alias));
    assert!(original.same_value_zero(&alias));
    assert_eq!(original, alias);

    // Aliases observe mutations through any handle, like JS references.
    original
        .as_object()
        .expect("object handle")
        .borrow_mut()
        .set("y", JsValue::Number(2.0));
    assert_eq!(
        alias.as_object().expect("object handle").borrow().get("y"),
        JsValue::Number(2.0)
    );
}

#[test]
fn aliased_callables_share_identity_and_hash() {
    let invocations = Cell::new(0);
    let original = Callable::new(|()| invocations.set(invocations.get() + 1));
    let alias = original.clone();

    assert!(original.same_value_zero(&alias));
    assert_eq!(original.js_hash(), alias.js_hash());
    assert_eq!(invocations.get(), 0);
}

#[test]
fn callable_set_keys_deduplicate_aliases_not_distinct_implementations() {
    let original = Callable::new(|()| 7_i32);
    let alias = original.clone();
    let distinct = Callable::new(|()| 7_i32);
    assert!(!original.same_value_zero(&distinct));

    let values = JsSet::from_values([original.clone(), alias.clone(), distinct.clone()]);
    assert_eq!(values.len(), 2);
    assert!(values.has(&original));
    assert!(values.has(&alias));
    assert!(values.has(&distinct));
    assert!(values.delete(&alias));
    assert_eq!(values.len(), 1);
    assert!(!values.has(&original));
    assert!(values.has(&distinct));
}

#[test]
fn callable_identity_does_not_require_argument_or_result_traits() {
    struct NonCloneInput;
    struct NonCloneOutput;

    let original = Callable::new(|(_input,): (NonCloneInput,)| NonCloneOutput);
    let alias = original.clone();
    assert!(original.same_value_zero(&alias));
    assert_eq!(original.js_hash(), alias.js_hash());

    let values = JsSet::from_values([original, alias]);
    assert_eq!(values.len(), 1);
}

#[test]
fn callable_identity_operations_allocate_nothing_and_do_not_invoke() {
    let invocations = Cell::new(0);
    let original = Callable::new(|()| invocations.set(invocations.get() + 1));
    let alias = original.clone();
    let distinct = Callable::new(|()| invocations.set(invocations.get() + 1));

    let ((same, different, original_hash, alias_hash), counts) = allocation_counts::measure(|| {
        let measured_alias = black_box(&original).clone();
        (
            black_box(&measured_alias).same_value_zero(black_box(&alias)),
            black_box(&original).same_value_zero(black_box(&distinct)),
            black_box(&original).js_hash(),
            black_box(&alias).js_hash(),
        )
    });

    assert!(same);
    assert!(!different);
    assert_eq!(original_hash, alias_hash);
    assert_eq!(counts.allocations, 0);
    assert_eq!(counts.deallocations, 0);
    assert_eq!(counts.reallocations, 0);
    assert_eq!(counts.requested_bytes(), 0);
    assert_eq!(invocations.get(), 0);
}

#[test]
fn structurally_identical_arrays_are_not_equal() {
    let left = sample_array();
    let right = sample_array();
    assert!(!left.strict_equal(&right));
    assert!(!left.same_value_zero(&right));
    assert_ne!(left, right);
}

#[test]
fn aliased_array_handles_are_identical() {
    let original = sample_array();
    let alias = original.clone();
    assert!(original.strict_equal(&alias));
    assert!(original.same_value_zero(&alias));
    assert_eq!(original, alias);

    original
        .as_array()
        .expect("array handle")
        .push(JsValue::Bool(true));
    assert_eq!(alias.as_array().expect("array handle").len(), 3);
}

#[test]
fn nan_and_signed_zero_semantics() {
    let nan = JsValue::Number(f64::NAN);
    assert!(!nan.strict_equal(&nan));
    assert!(nan.same_value_zero(&nan));

    let positive_zero = JsValue::Number(0.0);
    let negative_zero = JsValue::Number(-0.0);
    assert!(positive_zero.strict_equal(&negative_zero));
    assert!(positive_zero.same_value_zero(&negative_zero));
}

#[test]
fn map_keys_use_object_identity() {
    let first = sample_object();
    let second = sample_object();
    let map = JsMap::<JsValue, i32>::new();
    map.set(first.clone(), 1);
    map.set(second.clone(), 2);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&first), Some(1));
    assert_eq!(map.get(&second), Some(2));

    // The same handle is the same key.
    map.set(first.clone(), 3);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&first.clone()), Some(3));
}

#[test]
fn set_values_use_object_identity() {
    let first = sample_object();
    let second = sample_object();
    let set = JsSet::<JsValue>::new();
    set.add(first.clone());
    set.add(second);
    assert_eq!(set.len(), 2);

    // Adding an alias of an existing handle does not grow the set.
    set.add(first.clone());
    assert_eq!(set.len(), 2);
    assert!(set.has(&first));
}

#[test]
fn strings_compare_by_value() {
    let left = JsValue::Utf16String(JsString::from_utf8("héllo"));
    let right = JsValue::Utf16String(JsString::from_utf8("héllo"));
    assert!(left.strict_equal(&right));
    assert!(left.same_value_zero(&right));
    assert_eq!(left, right);
    assert_ne!(left, JsValue::Utf16String(JsString::from_utf8("other")));
}

#[test]
fn float32_bigint_and_undefined_follow_primitive_equality() {
    assert!(f32::NAN.same_value_zero(&f32::NAN));
    assert!(!f32::NAN.strict_equal(&f32::NAN));
    assert!(0.0_f32.strict_equal(&-0.0_f32));

    let left = BigInt::from_decimal_literal("9007199254740993");
    let right = BigInt::from_decimal_literal("9007199254740993");
    assert!(left.same_value_zero(&right));
    assert!(left.strict_equal(&right));
    assert!(().same_value_zero(&()));
    assert!(().strict_equal(&()));
}
