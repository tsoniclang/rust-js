use tsonic_rust_js::array::{statics, JsArray, JsSlot};

#[test]
fn sparse_array_length_delete_and_holes() {
    assert_eq!(JsSlot::Present(1).as_ref(), Some(&1));
    assert_eq!(JsSlot::<i32>::Hole.as_ref(), None);
    let xs = JsArray::from_dense(vec![1, 2]);
    xs.set_len(5);
    assert_eq!(xs.len(), 5);
    assert!(xs.has_index(1));
    assert!(!xs.has_index(3));
    assert!(xs.delete_at(1));
    assert!(xs.delete_at(1));
    assert!(xs.delete_at(100));
    assert_eq!(xs.len(), 5);
    assert!(!xs.has_index(1));
    assert_eq!(xs.get(1), None);
}

#[test]
fn sparse_array_mutation_helpers_preserve_holes() {
    let xs = JsArray::with_length(4);
    xs.set(0, 1);
    xs.set(2, 3);
    xs.fill(9, 1, Some(3));
    assert_eq!(xs.values(), vec![Some(1), Some(9), Some(9), None]);

    xs.delete_at(1);
    xs.copy_within(2, 0, Some(2));
    assert_eq!(xs.values(), vec![Some(1), None, Some(1), None]);

    xs.reverse();
    assert_eq!(xs.values(), vec![None, Some(1), None, Some(1)]);
}

#[test]
fn sparse_array_splice_shift_unshift_and_entries() {
    let xs = JsArray::from_dense(vec![1, 2, 3]);
    let removed = xs.splice(1, 1, vec![9, 10]);
    assert_eq!(removed.values(), vec![Some(2)]);
    assert_eq!(xs.values(), vec![Some(1), Some(9), Some(10), Some(3)]);
    assert_eq!(xs.shift(), Some(1));
    assert_eq!(xs.unshift(0), 4);
    assert_eq!(xs.pop(), Some(3));
    assert_eq!(xs.keys(), vec![0, 1, 2]);
    assert_eq!(
        xs.entries(),
        vec![(0, Some(0)), (1, Some(9)), (2, Some(10))]
    );
}

#[test]
fn js_array_at_supports_negative_indices_and_holes() {
    let values: JsArray<f64> = JsArray::from_dense(vec![1.0, 2.0, 3.0]);
    values.set_len(5);
    assert_eq!(values.at(0.0), Some(1.0));
    assert_eq!(values.at(-1.0), None);
    assert_eq!(values.at(-5.0), Some(1.0));
    assert_eq!(values.at(-3.0), Some(3.0));
    assert_eq!(values.at(5.0), None);
    assert_eq!(values.at(-6.0), None);
    values.set(4, 9.0);
    assert_eq!(values.at(-1.0), Some(9.0));
    assert_eq!(values.at(1.9), Some(2.0));
    assert_eq!(values.at(f64::NAN), Some(1.0));
    assert_eq!(values.at(f64::INFINITY), None);
    assert_eq!(values.at(f64::NEG_INFINITY), None);
}

#[test]
fn sparse_array_enumerable_keys_include_only_present_own_indices() {
    let values = JsArray::with_length(5);
    values.set(3, 4);
    values.set(1, 2);
    assert_eq!(values.enumerable_own_keys(), vec!["1", "3"]);
    values.delete_at(1);
    assert_eq!(values.enumerable_own_keys(), vec!["3"]);
}

#[test]
fn dense_and_sparse_arrays_share_reference_identity() {
    let dense = JsArray::from_dense(vec![1]);
    let dense_alias = dense.clone();
    dense_alias.push(2);
    assert!(dense.ptr_eq(&dense_alias));
    assert_eq!(dense.values(), vec![Some(1), Some(2)]);

    let sparse = JsArray::from_sparse(3, vec![(1, 4)]);
    let sparse_alias = sparse.clone();
    sparse_alias.set(2, 5);
    assert!(sparse.ptr_eq(&sparse_alias));
    assert_eq!(sparse.values(), vec![None, Some(4), Some(5)]);
}

#[test]
fn canonical_array_receiver_entrypoints_preserve_js_results() {
    let values = JsArray::from_dense(vec![1, 2, 3]);

    assert_eq!(values.iter_values().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert!(values.includes_from_start(&2));
    assert_eq!(values.index_of_from_start(&3), 2);
    assert_eq!(values.join_default(), "1,2,3");
    assert_eq!(values.slice_all().values(), values.values());
    assert_eq!(values.slice_from(1.0).values(), vec![Some(2), Some(3)]);
    assert_eq!(values.reduce(0, |sum, value| sum + value), 6);
    assert_eq!(
        values.to_reversed().values(),
        vec![Some(3), Some(2), Some(1)]
    );

    let sortable = JsArray::from_dense(vec![10, 2, 1]);
    sortable.sort_by_js_string();
    assert_eq!(sortable.values(), vec![Some(1), Some(10), Some(2)]);

    assert!(statics::is_array_value(&tsonic_rust_js::JsValue::from(
        vec![tsonic_rust_js::JsValue::Number(1.0)]
    )));
    assert!(!statics::is_array_value(&tsonic_rust_js::JsValue::Null));
}
