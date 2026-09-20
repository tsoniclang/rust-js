use std::cell::Cell;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use tsonic_rust_js::{json, regexp, string, JsArray, JsMap, JsObject, JsString, JsValue, Uint8Array};
use tsonic_rust_js::promise::JsPromise;

#[test]
fn native_strings_keep_bytes_and_explicit_utf16_remains_distinct() {
    let text = String::from("café😀");
    let address = text.as_ptr();
    let value = JsValue::from(text);
    let JsValue::String(text) = &value else { panic!("native string was converted"); };
    assert_eq!(text.as_ptr(), address);
    assert_eq!(string::js_len(text), 9);
    assert_eq!(string::index_of_from_start(text, "😀"), 5);
    assert!(string::slice_to(text, 0.0, 4.0).is_err());
    assert_eq!(json::parse("\"café😀\"").unwrap(), value);
    assert_eq!(json::stringify(&value).unwrap().unwrap(), "\"café😀\"");
    let exact = JsValue::from(JsString::from_units(vec![0xd800]));
    assert_eq!(json::stringify(&exact).unwrap().unwrap(), "\"\\ud800\"");
    assert!(json::parse("\"\\ud800\"").is_err());
}

#[test]
fn object_native_lookups_and_explicit_keys_share_one_identity() {
    let mut object = JsObject::new();
    object.set("café😀", 1.0);
    object.set_exact(JsString::from_utf8("café😀"), 2.0);
    assert_eq!(object.get("café😀"), JsValue::Number(2.0));
    assert_eq!(object.keys().unwrap(), ["café😀"]);
    let exact = JsString::from_units(vec![0xd800]);
    object.set_exact(exact.clone(), 3.0);
    assert_eq!(object.get_exact(&exact), JsValue::Number(3.0));
    assert!(object.keys().is_err());
    assert_eq!(json::stringify(&JsValue::object(object.clone())).unwrap().unwrap(), "{\"café😀\":2,\"\\ud800\":3}");
    assert!(object.has_exact_own_property(&exact));
    assert!(object.delete_exact(&exact));
    assert!(!object.has_exact_own_property(&exact));
}

#[test]
fn dense_arrays_initialize_and_keep_optional_values_explicit() {
    let values = JsArray::<i32>::with_length(4);
    assert_eq!(values.values(), vec![Some(0); 4]);
    values.set(4, 7);
    let alias = values.clone();
    alias.set(0, 9);
    assert_eq!(values.get(0), Some(9));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| values.set(7, 1))).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| values.delete_at(0))).is_err());
    assert_eq!(values.len(), 5);
    let reserved = JsArray::<String>::with_capacity(100);
    assert_eq!(reserved.len(), 0);
    let optional = JsArray::from_dense(vec![None::<String>]);
    assert!(optional.has_index(0));
    assert_eq!(optional.get(0), Some(None));
}

#[test]
fn deleted_map_values_drop_even_with_live_iteration() {
    let token = Rc::new(());
    let values = JsMap::new();
    values.set(1, token.clone());
    assert_eq!(Rc::strong_count(&token), 2);
    values.delete(&1);
    assert_eq!(Rc::strong_count(&token), 1);
    for index in 0..1000 { values.set(index, token.clone()); }
    values.clear();
    assert_eq!(Rc::strong_count(&token), 1);
    assert_eq!(values.len(), 0);
}

#[derive(Debug)]
struct Counted(Rc<Cell<usize>>);

impl Clone for Counted {
    fn clone(&self) -> Self { self.0.set(self.0.get() + 1); Self(self.0.clone()) }
}

#[test]
fn unique_promise_moves_values_and_shared_promise_remains_repeatable() {
    let copies = Rc::new(Cell::new(0));
    let promise = JsPromise::resolved(Counted(copies.clone()));
    let mut future = std::pin::pin!(promise.into_result());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(std::future::Future::poll(future.as_mut(), &mut context), Poll::Ready(Ok(_))));
    assert_eq!(copies.get(), 0);
    let promise = JsPromise::resolved(String::from("retained"));
    let alias = promise.clone();
    let mut first = std::pin::pin!(promise.into_result());
    let mut second = std::pin::pin!(alias.into_result());
    assert!(matches!(std::future::Future::poll(first.as_mut(), &mut context), Poll::Ready(Ok(value)) if value == "retained"));
    assert!(matches!(std::future::Future::poll(second.as_mut(), &mut context), Poll::Ready(Ok(value)) if value == "retained"));
}

#[test]
fn native_regex_uses_byte_offsets_and_dense_optional_captures() {
    let expression = regexp::regexp_new_native("😀", "g").unwrap();
    assert!(regexp::regexp_test_native(&expression, "é😀").unwrap());
    assert_eq!(expression.last_index(), 6.0);
    assert!(!regexp::regexp_test_native(&expression, "é😀").unwrap());
    assert_eq!(expression.last_index(), 0.0);
    let expression = regexp::regexp_new_native("(a)?b", "").unwrap();
    let parts = regexp::regexp_split_all_native(&expression, "xbz").unwrap();
    assert_eq!(parts.get(1), Some(None));
    assert!(parts.has_index(1));
    assert_eq!(parts.get(2), Some(Some(String::from("z"))));
}

#[test]
fn bulk_typed_array_copy_overlaps_and_fill_retains_views() {
    let first = tsonic_rust_js::ArrayBuffer::from_bytes(vec![1, 2, 3]);
    let second = tsonic_rust_js::ArrayBuffer::from_bytes(vec![2, 3]);
    assert!(first.with_byte_ranges(1..3, &second, 0..2, |left, right| left == right));
    assert!(second.with_byte_ranges(0..2, &first, 1..3, |left, right| left == right));
    assert!(first.with_byte_ranges(1..3, &first.clone(), 1..3, |left, right| left == right));
    let values = Uint8Array::from_bytes(vec![1, 2, 3, 4, 5]);
    values.set_from_typed_array(&values.subarray(0.0, Some(4.0)), 1.0).unwrap();
    assert_eq!(values.with_bytes(<[u8]>::to_vec), [1, 1, 2, 3, 4]);
    values.subarray(1.0, Some(4.0)).fill(7.0, 0.0, None);
    assert_eq!(values.with_bytes(<[u8]>::to_vec), [1, 7, 7, 7, 4]);
}
