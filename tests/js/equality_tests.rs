use tsonic_rust_js::equality::{JsSameValueZero, JsStrictEqual};
use tsonic_rust_js::{JsMap, JsObject, JsSet, JsValue};

fn sample_object() -> JsValue {
    JsValue::object(JsObject::from_pairs([("x", JsValue::Number(1.0))]))
}

fn sample_array() -> JsValue {
    JsValue::from(vec![JsValue::Number(1.0), JsValue::String("a".to_string())])
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
        .borrow_mut()
        .push(JsValue::Bool(true));
    assert_eq!(alias.as_array().expect("array handle").borrow().len(), 3);
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
    let mut map = JsMap::<JsValue, i32>::new();
    map.set(first.clone(), 1);
    map.set(second.clone(), 2);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&first), Some(&1));
    assert_eq!(map.get(&second), Some(&2));

    // The same handle is the same key.
    map.set(first.clone(), 3);
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&first.clone()), Some(&3));
}

#[test]
fn set_values_use_object_identity() {
    let first = sample_object();
    let second = sample_object();
    let mut set = JsSet::<JsValue>::new();
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
    let left = JsValue::String("héllo".to_string());
    let right = JsValue::String("héllo".to_string());
    assert!(left.strict_equal(&right));
    assert!(left.same_value_zero(&right));
    assert_eq!(left, right);
    assert_ne!(left, JsValue::String("other".to_string()));
}
