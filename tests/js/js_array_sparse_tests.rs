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
    xs.fill_to(9, 1.0, 3.0);
    assert_eq!(xs.values(), vec![Some(1), Some(9), Some(9), None]);

    xs.delete_at(1);
    xs.copy_within_to(2.0, 0.0, 2.0);
    assert_eq!(xs.values(), vec![Some(1), None, Some(1), None]);

    xs.reverse();
    assert_eq!(xs.values(), vec![None, Some(1), None, Some(1)]);
}

#[test]
fn sparse_array_splice_shift_unshift_and_entries() {
    let xs = JsArray::from_dense(vec![1, 2, 3]);
    let removed = xs.splice_many(1.0, 1.0, [9, 10]);
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
fn variadic_mutations_move_values_in_source_order_and_preserve_identity() {
    let values = JsArray::from_dense(vec![2]);
    let alias = values.clone();

    assert_eq!(values.unshift_many([0, 1]), 3);
    assert_eq!(values.push_many([3, 4]), 5);
    assert_eq!(values.push_many([]), 5);
    assert_eq!(
        values.values(),
        vec![Some(0), Some(1), Some(2), Some(3), Some(4)]
    );

    let filled = values.fill_to(9, -3.9, f64::INFINITY);
    assert!(values.ptr_eq(&filled));
    assert_eq!(
        alias.values(),
        vec![Some(0), Some(1), Some(9), Some(9), Some(9)]
    );

    let copied = values.copy_within_from(-2.0, 0.0);
    assert!(values.ptr_eq(&copied));
    assert_eq!(
        values.values(),
        vec![Some(0), Some(1), Some(9), Some(0), Some(1)]
    );

    let reversed = values.reverse();
    assert!(values.ptr_eq(&reversed));
    assert_eq!(
        values.values(),
        vec![Some(1), Some(0), Some(9), Some(1), Some(0)]
    );

    let filled_all = values.fill_all(6);
    assert!(values.ptr_eq(&filled_all));
    assert_eq!(
        values.values(),
        vec![Some(6), Some(6), Some(6), Some(6), Some(6)]
    );

    let filled_from = values.fill_from(7, -2.0);
    assert!(values.ptr_eq(&filled_from));
    assert_eq!(
        values.values(),
        vec![Some(6), Some(6), Some(6), Some(7), Some(7)]
    );
}

#[test]
fn splice_uses_js_numeric_bounds_and_returns_a_distinct_removed_array() {
    let values = JsArray::from_dense(vec![0, 1, 2, 3]);
    let removed = values.splice_many(-3.8, 1.9, [8, 9]);
    assert!(!values.ptr_eq(&removed));
    assert_eq!(removed.values(), vec![Some(1)]);
    assert_eq!(
        values.values(),
        vec![Some(0), Some(8), Some(9), Some(2), Some(3)]
    );

    let tail = values.splice_from(3.0);
    assert_eq!(tail.values(), vec![Some(2), Some(3)]);
    assert_eq!(values.values(), vec![Some(0), Some(8), Some(9)]);

    let none = values.splice_many(f64::NAN, f64::NAN, []);
    assert!(none.is_empty());
    assert_eq!(values.values(), vec![Some(0), Some(8), Some(9)]);
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

#[test]
fn array_search_indexes_follow_ecmascript_number_rules() {
    let values = JsArray::from_dense(vec![1, 2, 1, 2]);

    assert!(values.includes(&1, f64::NEG_INFINITY));
    assert!(!values.includes(&1, f64::INFINITY));
    assert_eq!(values.index_of(&2, 1.9), 1);
    assert_eq!(values.index_of(&1, -2.0), 2);
    assert_eq!(values.last_index_of_from_end(&2), 3);
    assert_eq!(values.last_index_of(&2, -2.0), 1);
    assert_eq!(values.last_index_of(&1, f64::NAN), 0);
    assert_eq!(values.last_index_of(&1, f64::NEG_INFINITY), -1);
}

#[test]
fn default_array_sort_compares_utf16_code_units() {
    let values = JsArray::from_dense(vec!["\u{10000}".to_string(), "\u{e000}".to_string()]);
    values.sort_by_js_string();
    assert_eq!(
        values.values(),
        vec![Some("\u{10000}".to_string()), Some("\u{e000}".to_string())]
    );
}
