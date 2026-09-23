use tsonic_rust_js::array::{statics, JsArray};

#[test]
fn array_search_borrows_string_queries_without_materializing_owned_values() {
    let values = JsArray::with_length(5);
    values.set(1, String::from("café😀"));
    values.set(3, String::from("café😀"));
    let query = String::from("café😀");
    assert!(values.includes_from_start(query.as_str()));
    assert!(values.includes_from_start(&query));
    assert!(values.includes("café😀", 2.0));
    assert!(!values.includes("café😀", 4.0));
    assert_eq!(values.index_of_from_start("café😀"), 1);
    assert_eq!(values.index_of("café😀", 2.0), 3);
    assert_eq!(values.last_index_of_from_end("café😀"), 3);
    assert_eq!(values.last_index_of("café😀", -3.0), 1);
    assert!(!values.includes_from_start("missing"));
    assert_eq!(values.index_of_from_start(""), 0);
    assert_eq!(values.last_index_of_from_end("missing"), -1);
    assert_eq!(values.get(1).as_deref(), Some("café😀"));
}

#[test]
fn object_keys_returns_an_independent_dense_array_without_cloning_source_values() {
    struct Token;
    let values = JsArray::from_dense(vec![Token, Token, Token]);
    values.set_number(-1.0, Token);
    values.set_number(f64::NAN, Token);
    let keys = values.object_keys();
    assert_eq!(keys.join("|"), "0|1|2|-1|NaN");
    assert_eq!(keys.len(), 5);
    assert!((0..keys.len()).all(|index| keys.has_index(index)));
    values.set_len(1);
    values.delete_number(-1.0);
    values.set_number(-1.0, Token);
    assert_eq!(values.object_keys().join("|"), "0|NaN|-1");
    assert_eq!(keys.join("|"), "0|1|2|-1|NaN");
}

#[test]
fn numeric_membership_preserves_presence_without_cloning_values() {
    struct Token;
    let values = JsArray::from_dense(vec![Token]);
    assert!(JsArray::contains_number_property(-0.0, &values));
    assert!(!JsArray::contains_number_property(1.0, &values));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| values.set_len(3))).is_err());
    assert!(!JsArray::contains_number_property(2.0, &values));
    for index in [-1.0, 0.5, f64::NAN, f64::INFINITY, 4_294_967_295.0] {
        assert!(!JsArray::contains_number_property(index, &values));
        values.set_number(index, Token);
        assert!(JsArray::contains_number_property(index, &values));
        values.delete_number(index);
        assert!(!JsArray::contains_number_property(index, &values));
    }
    values.set_len(0);
    assert!(!JsArray::contains_number_property(0.0, &values));
    let optional = JsArray::from_dense(vec![None::<Token>]);
    assert!(JsArray::contains_number_property(0.0, &optional));
    optional.set_len(0);
    assert!(!JsArray::contains_number_property(0.0, &optional));
    optional.set(0, None);
    assert!(JsArray::contains_number_property(0.0, &optional));
}

#[test]
fn dense_array_rejects_hole_creation_without_mutation() {
    let xs = JsArray::from_dense(vec![1, 2]);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| xs.set_len(5))).is_err());
    assert_eq!(xs.len(), 2);
    assert!(xs.has_index(1));
    assert!(!xs.has_index(3));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| xs.delete_at(1))).is_err());
    assert!(xs.delete_at(100));
    assert_eq!(xs.len(), 2);
    assert!(xs.has_index(1));
    assert_eq!(xs.get(1), Some(2));
}

#[test]
fn number_indexes_preserve_array_slots_and_non_index_properties() {
    let values = JsArray::from_dense(vec![10, 20]);

    assert_eq!(values.get_number(1.0), Some(20));
    assert_eq!(values.get_number(-0.0), Some(10));
    assert_eq!(values.get_number(2.5), None);

    values.set_number(2.0, 30);
    values.set_number(2.5, 25);
    values.set_number(f64::NAN, 99);
    assert_eq!(values.values(), vec![10, 20, 30]);
    assert_eq!(values.get_number(2.5), Some(25));
    assert_eq!(values.get_number(f64::NAN), Some(99));
    assert_eq!(
        values.enumerable_own_keys(),
        vec!["0", "1", "2", "2.5", "NaN"],
    );

    assert!(values.delete_number(2.5));
    assert_eq!(values.get_number(2.5), None);
    assert_eq!(values.len(), 3);
}

#[test]
fn dense_array_mutation_helpers_preserve_initialized_values() {
    let xs = JsArray::with_length(4);
    xs.set(0, 1);
    xs.set(2, 3);
    xs.fill_to(9, 1.0, 3.0);
    assert_eq!(xs.values(), vec![1, 9, 9, 0]);

    xs.set(1, 0);
    xs.copy_within_to(2.0, 0.0, 2.0);
    assert_eq!(xs.values(), vec![1, 0, 1, 0]);

    xs.reverse();
    assert_eq!(xs.values(), vec![0, 1, 0, 1]);
}

#[test]
fn dense_array_splice_shift_unshift_and_entries() {
    let xs = JsArray::from_dense(vec![1, 2, 3]);
    let removed = xs.splice_many(1.0, 1.0, [9, 10]);
    assert_eq!(removed.values(), vec![2]);
    assert_eq!(xs.values(), vec![1, 9, 10, 3]);
    assert_eq!(xs.shift(), Some(1));
    assert_eq!(xs.unshift(0), 4);
    assert_eq!(xs.pop(), Some(3));
    assert_eq!(xs.keys(), vec![0, 1, 2]);
    assert_eq!(
        xs.entries().collect::<Vec<_>>(),
        vec![(0, 0), (1, 9), (2, 10)]
    );
}

#[test]
fn variadic_mutations_move_values_in_source_order_and_preserve_identity() {
    let values = JsArray::from_dense(vec![2]);
    let alias = values.clone();

    assert_eq!(values.unshift_many([0, 1]), 3);
    assert_eq!(values.push_many([3, 4]), 5);
    assert_eq!(values.push_many([]), 5);
    assert_eq!(values.values(), vec![0, 1, 2, 3, 4]);

    let filled = values.fill_to(9, -3.9, f64::INFINITY);
    assert!(values.ptr_eq(&filled));
    assert_eq!(alias.values(), vec![0, 1, 9, 9, 9]);

    let copied = values.copy_within_from(-2.0, 0.0);
    assert!(values.ptr_eq(&copied));
    assert_eq!(values.values(), vec![0, 1, 9, 0, 1]);

    let reversed = values.reverse();
    assert!(values.ptr_eq(&reversed));
    assert_eq!(values.values(), vec![1, 0, 9, 1, 0]);

    let filled_all = values.fill_all(6);
    assert!(values.ptr_eq(&filled_all));
    assert_eq!(values.values(), vec![6, 6, 6, 6, 6]);

    let filled_from = values.fill_from(7, -2.0);
    assert!(values.ptr_eq(&filled_from));
    assert_eq!(values.values(), vec![6, 6, 6, 7, 7]);
}

#[test]
fn discarded_variadic_mutations_preserve_order_and_shared_identity() {
    let values = JsArray::from_dense(vec![2]);
    let alias = values.clone();

    values.unshift_many_discard([0, 1]);
    values.push_many_discard([3, 4]);
    values.push_many_discard([]);

    assert_eq!(alias.values(), vec![0, 1, 2, 3, 4]);
    assert!(values.ptr_eq(&alias));
}

#[test]
fn variadic_sequences_move_non_clone_values_and_reuse_owned_storage() {
    struct Token(usize);
    let items = vec![Token(1), Token(2)];
    let original = items.as_ptr();
    let values = statics::of(items);
    assert_eq!(
        values.borrow_number_element(0).as_deref().unwrap() as *const Token,
        original
    );
    let alias = values.clone();
    assert_eq!(values.push_many(vec![Token(3), Token(4)]), 4);
    assert_eq!(values.unshift_many(vec![Token(0)]), 5);
    values.push_many_discard(vec![Token(5)]);
    values.unshift_many_discard(std::iter::empty());
    let removed = values.splice_many(1.0, 2.0, vec![Token(8), Token(9)]);
    assert_eq!(removed.borrow_number_element(0).unwrap().0, 1);
    assert_eq!(removed.borrow_number_element(1).unwrap().0, 2);
    assert_eq!(alias.len(), 6);
    for (index, expected) in [0, 8, 9, 3, 4, 5].into_iter().enumerate() {
        assert_eq!(
            alias.borrow_number_element(index as f64).unwrap().0,
            expected
        );
    }
}

#[test]
fn splice_uses_js_numeric_bounds_and_returns_a_distinct_removed_array() {
    let values = JsArray::from_dense(vec![0, 1, 2, 3]);
    let removed = values.splice_many(-3.8, 1.9, [8, 9]);
    assert!(!values.ptr_eq(&removed));
    assert_eq!(removed.values(), vec![1]);
    assert_eq!(values.values(), vec![0, 8, 9, 2, 3]);

    let tail = values.splice_from(3.0);
    assert_eq!(tail.values(), vec![2, 3]);
    assert_eq!(values.values(), vec![0, 8, 9]);

    let none = values.splice_many(f64::NAN, f64::NAN, []);
    assert!(none.is_empty());
    assert_eq!(values.values(), vec![0, 8, 9]);
}

#[test]
fn js_array_at_supports_negative_indices_and_initialized_growth() {
    let values: JsArray<f64> = JsArray::from_dense(vec![1.0, 2.0, 3.0]);
    values.push_many([0.0, 0.0]);
    assert_eq!(values.at(0.0), Some(1.0));
    assert_eq!(values.at(-1.0), Some(0.0));
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
fn dense_array_enumerable_keys_include_all_initialized_indices() {
    let values = JsArray::with_length(5);
    values.set(3, 4);
    values.set(1, 2);
    assert_eq!(values.enumerable_own_keys(), vec!["0", "1", "2", "3", "4"]);
    values.set_len(2);
    assert_eq!(values.enumerable_own_keys(), vec!["0", "1"]);
}

#[test]
fn dense_arrays_share_reference_identity() {
    let dense = JsArray::from_dense(vec![1]);
    let dense_alias = dense.clone();
    dense_alias.push(2);
    assert!(dense.ptr_eq(&dense_alias));
    assert_eq!(dense.values(), vec![1, 2]);

    let initialized = JsArray::with_length(3);
    initialized.set(1, 4);
    let alias = initialized.clone();
    alias.set(2, 5);
    assert!(initialized.ptr_eq(&alias));
    assert_eq!(initialized.values(), vec![0, 4, 5]);
}

#[test]
fn canonical_array_receiver_entrypoints_preserve_js_results() {
    let values = JsArray::from_dense(vec![1, 2, 3]);

    assert_eq!(values.iter_values().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert!(values.includes_from_start(&2));
    assert_eq!(values.index_of_from_start(&3), 2);
    assert_eq!(values.join_default(), "1,2,3");
    assert_eq!(values.slice_all().values(), values.values());
    assert_eq!(values.slice_from(1.0).values(), vec![2, 3]);
    assert_eq!(values.reduce(0, |sum, value| sum + value), 6);
    assert_eq!(values.to_reversed().values(), vec![3, 2, 1]);

    let sortable = JsArray::from_dense(vec![10, 2, 1]);
    sortable.sort_by_js_string();
    assert_eq!(sortable.values(), vec![1, 10, 2]);

    assert!(statics::is_array_value(&tsonic_rust_js::JsValue::from(
        vec![tsonic_rust_js::JsValue::Number(1.0)]
    )));
    assert!(!statics::is_array_value(&tsonic_rust_js::JsValue::Null));
}

#[test]
fn array_join_formats_floating_numbers_with_javascript_semantics() {
    let values = JsArray::from_dense(vec![
        2.0_f64,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]);

    assert_eq!(values.join(","), "2,0,NaN,Infinity,-Infinity");

    let single_precision = JsArray::from_dense(vec![2.5_f32, -0.0]);
    assert_eq!(single_precision.join(","), "2.5,0");
}

#[test]
fn array_static_factories_preserve_values_and_array_brand() {
    let values = statics::of([1, 2, 3]);
    assert_eq!(values.values(), vec![1, 2, 3]);

    let vector = vec![2, 4, 6];
    assert_eq!(statics::from_vec(&vector).values(), vec![2, 4, 6]);
    assert_eq!(
        statics::from_vec_map_zero(&vector, || 7).values(),
        vec![7, 7, 7]
    );
    assert_eq!(
        statics::from_vec_map(&vector, |value| value / 2).values(),
        vec![1, 2, 3]
    );
    assert_eq!(
        statics::from_vec_map_with_index(&vector, |value, index| value + index as i32).values(),
        vec![2, 5, 8]
    );

    let text = statics::from_string("a😀");
    assert_eq!(text.values(), vec!["a".to_owned(), "😀".to_owned()]);

    assert!(statics::is_array(&values));
    assert!(statics::is_array_value(&tsonic_rust_js::JsValue::from(
        vec![tsonic_rust_js::JsValue::Number(1.0)]
    )));
    assert!(!statics::is_array_value(&tsonic_rust_js::JsValue::Null));
}

#[test]
fn mapped_string_construction_preserves_scalars_indices_and_empty_input() {
    let input = "a😀é";
    assert_eq!(
        statics::from_string_map_zero(input, || 7).values(),
        vec![7; 3]
    );
    assert_eq!(
        statics::from_string_map(input, |part| part).values(),
        vec!["a".to_owned(), "😀".to_owned(), "é".to_owned()]
    );
    assert_eq!(
        statics::from_string_map_with_index(input, |part, index| format!("{part}:{index}"))
            .values(),
        vec!["a:0".to_owned(), "😀:1".to_owned(), "é:2".to_owned()]
    );
    let mut calls = 0;
    let empty = statics::from_string_map_zero("", || {
        calls += 1;
        calls
    });
    assert_eq!(empty.len(), 0);
    assert_eq!(calls, 0);
    let exact = statics::from_string_try_map("Aÿ", |part| {
        u8::try_from(part.chars().next().unwrap() as u32)
    });
    assert_eq!(exact.unwrap().values(), vec![65, 255]);
}

#[test]
fn mapped_string_construction_stops_at_each_callback_failure() {
    let mut zero_calls = 0;
    let zero = statics::from_string_try_map_zero("abc", || {
        zero_calls += 1;
        if zero_calls == 2 {
            Err("stop")
        } else {
            Ok(zero_calls)
        }
    });
    assert_eq!(zero.unwrap_err(), "stop");
    assert_eq!(zero_calls, 2);
    let mut visited = String::new();
    let values = statics::from_string_try_map("a😀z", |part| {
        visited.push_str(&part);
        if part == "😀" {
            Err("stop")
        } else {
            Ok(part)
        }
    });
    assert_eq!(values.unwrap_err(), "stop");
    assert_eq!(visited, "a😀");
    let mut indices = Vec::new();
    let indexed = statics::from_string_try_map_with_index("a😀z", |part, index| {
        indices.push(index);
        if index == 1 {
            Err("stop")
        } else {
            Ok(part)
        }
    });
    assert_eq!(indexed.unwrap_err(), "stop");
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn concat_preserves_values_order_and_source_identity() {
    let left = JsArray::from_dense(vec![1, 0, 3]);
    let right = JsArray::from_dense(vec![0, 5]);

    let joined = left.concat([
        statics::JsArrayConcatItem::Value(4),
        statics::JsArrayConcatItem::Array(right.clone()),
    ]);

    assert_eq!(joined.len(), 6);
    assert_eq!(joined.get(0), Some(1));
    assert_eq!(joined.get(1), Some(0));
    assert_eq!(joined.get(2), Some(3));
    assert_eq!(joined.get(3), Some(4));
    assert_eq!(joined.get(4), Some(0));
    assert_eq!(joined.get(5), Some(5));

    left.set(0, 9);
    right.set(1, 8);
    assert_eq!(joined.get(0), Some(1));
    assert_eq!(joined.get(5), Some(5));
}

#[test]
fn array_callbacks_receive_exact_declared_argument_shapes() {
    let values = JsArray::from_dense(vec![2, 4, 6]);
    let alias = values.clone();

    let mapped = values.map_with_array(|value, index, array| {
        assert!(array.ptr_eq(&alias));
        value + index as i32
    });
    assert_eq!(mapped.values(), vec![2, 5, 8]);

    let filtered =
        values.filter_with_index(|value, index| value > i32::try_from(index + 1).unwrap());
    assert_eq!(filtered.values(), vec![2, 4, 6]);

    let mut visits = Vec::new();
    values.for_each(|value, index, array| {
        assert!(array.ptr_eq(&alias));
        visits.push((value, index));
    });
    assert_eq!(visits, vec![(2, 0), (4, 1), (6, 2)]);

    assert_eq!(
        values.reduce_with_array(0, |sum, value, index, array| {
            assert!(array.ptr_eq(&alias));
            sum + value + index as i32
        }),
        15
    );
    assert_eq!(
        values
            .reduce_from_first_with_index(|sum, value, index| sum + value + index as i32)
            .expect("non-empty array reduces"),
        15
    );
}

#[test]
fn every_array_callback_arity_has_executable_runtime_coverage() {
    let values = JsArray::from_dense(vec![2, 4, 6]);
    let alias = values.clone();

    let mut map_calls = 0;
    assert_eq!(
        values
            .map_zero(|| {
                map_calls += 1;
                map_calls
            })
            .values(),
        vec![1, 2, 3]
    );
    assert_eq!(
        values
            .map_with_index(|value, index| value + index as i32)
            .values(),
        vec![2, 5, 8]
    );
    assert_eq!(values.filter_zero(|| true).values(), values.values());
    assert_eq!(
        values
            .filter_with_array(|value, _, array| array.ptr_eq(&alias) && value > 2)
            .values(),
        vec![4, 6]
    );

    assert_eq!(values.reduce_zero(0, || 7), 7);
    assert_eq!(
        values.reduce_accumulator(1, |accumulator| accumulator + 1),
        4
    );
    assert_eq!(
        values.reduce_with_index(0, |sum, value, index| sum + value + index as i32),
        15
    );
    assert_eq!(values.reduce_from_first_zero(|| 9).unwrap(), 9);
    assert_eq!(
        values
            .reduce_from_first_accumulator(|accumulator| accumulator + 1)
            .unwrap(),
        4
    );
    assert_eq!(
        values
            .reduce_from_first_with_array(|sum, value, index, array| {
                assert!(array.ptr_eq(&alias));
                sum + value + index as i32
            })
            .unwrap(),
        15
    );

    let mut visits = Vec::new();
    values.for_each_value_index(|value, index| visits.push((value, index)));
    assert_eq!(visits, vec![(2, 0), (4, 1), (6, 2)]);

    assert_eq!(values.find_zero(|| true), Some(2));
    assert_eq!(
        values.find_with_index(|value, index| value == 4 && index == 1),
        Some(4)
    );
    assert_eq!(
        values.find_with_array(|value, _, array| array.ptr_eq(&alias) && value == 6),
        Some(6)
    );
    assert_eq!(values.find_index_zero(|| true), 0);
    assert_eq!(
        values.find_index_with_index(|value, index| value == 4 && index == 1),
        1
    );
    assert_eq!(
        values.find_index_with_array(|value, _, array| array.ptr_eq(&alias) && value == 6),
        2
    );

    assert_eq!(values.find_last_zero(|| true), Some(6));
    assert_eq!(
        values.find_last_with_index(|value, index| value == 4 && index == 1),
        Some(4)
    );
    assert_eq!(
        values.find_last_with_array(|value, _, array| array.ptr_eq(&alias) && value == 6),
        Some(6)
    );
    assert_eq!(values.find_last_index_zero(|| true), 2);
    assert_eq!(
        values.find_last_index_with_index(|value, index| value == 4 && index == 1),
        1
    );
    assert_eq!(
        values.find_last_index_with_array(|value, _, array| { array.ptr_eq(&alias) && value == 6 }),
        2
    );

    assert!(values.some_zero(|| true));
    assert!(values.some_with_index(|value, index| value == 6 && index == 2));
    assert!(values.some_with_array(|_, _, array| array.ptr_eq(&alias)));
    assert!(values.every_zero(|| true));
    assert!(values.every_with_index(|value, index| value >= i32::try_from(index).unwrap()));
    assert!(values.every_with_array(|_, _, array| array.ptr_eq(&alias)));
}

#[test]
fn array_reduce_without_initial_uses_first_value_and_rejects_empty_input() {
    let sparse = JsArray::from_dense(vec![0, 0, 4, 0, 6]);
    assert_eq!(
        sparse
            .reduce_from_first(|sum, value| sum + value)
            .expect("present values reduce"),
        10
    );

    assert_eq!(
        JsArray::<i32>::with_length(3)
            .reduce_from_first(|left, right| left + right)
            .unwrap(),
        0
    );
    let empty = JsArray::<i32>::new();
    let error = empty
        .reduce_from_first(|sum, value| sum + value)
        .expect_err("an empty array has no initial accumulator");
    assert_eq!(error.kind(), tsonic_rust_runtime::JsErrorKind::TypeError);
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
fn default_array_sort_compares_native_strings() {
    let values = JsArray::from_dense(vec!["\u{10000}".to_string(), "\u{e000}".to_string()]);
    values.sort_by_js_string();
    assert_eq!(
        values.values(),
        vec!["\u{e000}".to_string(), "\u{10000}".to_string()]
    );
}

#[test]
fn comparator_sort_entrypoints_preserve_callback_arity_and_identity() {
    let binary = JsArray::from_dense(vec![3, 1, 2]);
    let binary_alias = binary.clone();
    let sorted = binary.sort(|left, right| f64::from(left - right));
    assert!(sorted.ptr_eq(&binary_alias));
    assert_eq!(binary.values(), vec![1, 2, 3]);

    let unary = JsArray::from_dense(vec![3, 1, 2]);
    unary.sort_value(|left| f64::from(left - 2));
    assert_eq!(unary.len(), 3);

    let zero = JsArray::from_dense(vec![3, 1, 2]);
    zero.sort_zero(|| 0.0);
    assert_eq!(zero.values(), vec![3, 1, 2]);
}
