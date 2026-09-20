use tsonic_rust_js::JsArray;
use tsonic_rust_runtime::ObjectIdentityCarrier;

#[test]
fn array_entries_keep_a_shared_live_cursor_and_stay_complete() {
    let array = JsArray::with_length(3);
    array.set(1, 7);
    let mut first = array.entries();
    let mut alias = first.clone();
    assert_eq!(first.object_identity(), alias.object_identity());
    assert_ne!(first.object_identity(), array.entries().object_identity());
    assert_eq!(first.next(), Some((0.0, 0)));
    array.set(1, 9);
    assert_eq!(alias.next(), Some((1.0, 9)));
    array.push(11);
    assert_eq!(first.next(), Some((2.0, 0)));
    assert_eq!(alias.next(), Some((3.0, 11)));
    assert_eq!(first.next(), None);
    array.push(13);
    assert_eq!(alias.next(), None);
    assert!(first.next_result().done());
}

#[test]
fn array_entries_observe_shrinkage_and_empty_completion() {
    let array = JsArray::from_dense(vec!["first".to_string(), "second".to_string()]);
    let entries = array.entries();
    assert_eq!(
        entries.next_result().yield_value(),
        (0.0, "first".to_string())
    );
    array.set_len(0);
    assert!(entries.next_result().done());
    array.push("third".to_string());
    assert!(entries.next_result().done());
    let empty: JsArray<i32> = JsArray::new();
    let iterator = empty.entries();
    empty.push(5);
    assert_eq!(iterator.next_result().yield_value(), (0.0, 5));
}

#[test]
fn dense_entry_projection_preserves_stored_absence_and_live_cursor() {
    let array = JsArray::from_dense(vec![None, Some(7)]);
    let entries = array.entries();
    let mut projected = entries.clone();
    assert_eq!(projected.next(), Some((0.0, None)));
    array.push(Some(11));
    assert_eq!(entries.next_result().yield_value(), (1.0, Some(7)));
    assert_eq!(projected.next(), Some((2.0, Some(11))));
    assert_eq!(projected.next(), None);
    array.push(None);
    assert_eq!(projected.next(), None);
}

#[test]
fn dense_entry_projection_exposes_initialized_elements() {
    let array: JsArray<i32> = JsArray::with_length(1);
    assert_eq!(array.entries().next(), Some((0.0, 0)));
}
