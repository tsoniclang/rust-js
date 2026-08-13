use tsonic_rust_js::{JsArray, JsError, JsErrorKind};
use tsonic_rust_runtime::TsonicError;

#[test]
fn fallible_array_callback_entrypoints_cover_every_declared_arity() {
    let values = JsArray::from_dense(vec![1, 2, 3]);

    assert!(values.try_map_zero(|| Ok::<_, TsonicError>(1)).is_ok());
    assert!(values.try_map(|value| Ok::<_, TsonicError>(value)).is_ok());
    assert!(values
        .try_map_with_index(|value, index| Ok::<_, TsonicError>(value + index as i32))
        .is_ok());
    assert!(values
        .try_map_with_array(|value, _, array| { Ok::<_, TsonicError>(value + array.len() as i32) })
        .is_ok());

    assert!(values
        .try_filter_zero(|| Ok::<_, TsonicError>(true))
        .is_ok());
    assert!(values
        .try_filter(|value| Ok::<_, TsonicError>(value > 0))
        .is_ok());
    assert!(values
        .try_filter_with_index(|value, index| Ok::<_, TsonicError>(value as f64 > index))
        .is_ok());
    assert!(values
        .try_filter_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value <= array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_reduce_zero(0, || Ok::<_, TsonicError>(1))
        .is_ok());
    assert!(values
        .try_reduce_accumulator(0, |sum| Ok::<_, TsonicError>(sum + 1))
        .is_ok());
    assert!(values
        .try_reduce(0, |sum, value| Ok::<_, TsonicError>(sum + value))
        .is_ok());
    assert!(values
        .try_reduce_with_index(0, |sum, value, index| {
            Ok::<_, TsonicError>(sum + value + index as i32)
        })
        .is_ok());
    assert!(values
        .try_reduce_with_array(0, |sum, value, _, array| {
            Ok::<_, TsonicError>(sum + value + array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_reduce_from_first_zero(|| Ok::<_, TsonicError>(1))
        .is_ok());
    assert!(values
        .try_reduce_from_first_accumulator(|sum| Ok::<_, TsonicError>(sum + 1))
        .is_ok());
    assert!(values
        .try_reduce_from_first(|sum, value| Ok::<_, TsonicError>(sum + value))
        .is_ok());
    assert!(values
        .try_reduce_from_first_with_index(|sum, value, index| {
            Ok::<_, TsonicError>(sum + value + index as i32)
        })
        .is_ok());
    assert!(values
        .try_reduce_from_first_with_array(|sum, value, _, array| {
            Ok::<_, TsonicError>(sum + value + array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_for_each_zero(|| Ok::<_, TsonicError>(()))
        .is_ok());
    assert!(values
        .try_for_each_value(|_| Ok::<_, TsonicError>(()))
        .is_ok());
    assert!(values
        .try_for_each_value_index(|_, _| Ok::<_, TsonicError>(()))
        .is_ok());
    assert!(values
        .try_for_each(|_, _, _| Ok::<_, TsonicError>(()))
        .is_ok());

    assert!(values.try_find_zero(|| Ok::<_, TsonicError>(true)).is_ok());
    assert!(values
        .try_find(|value| Ok::<_, TsonicError>(value == 2))
        .is_ok());
    assert!(values
        .try_find_with_index(|value, index| Ok::<_, TsonicError>(value as f64 > index))
        .is_ok());
    assert!(values
        .try_find_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value == array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_find_index_zero(|| Ok::<_, TsonicError>(true))
        .is_ok());
    assert!(values
        .try_find_index(|value| Ok::<_, TsonicError>(value == 2))
        .is_ok());
    assert!(values
        .try_find_index_with_index(|value, index| { Ok::<_, TsonicError>(value as f64 > index) })
        .is_ok());
    assert!(values
        .try_find_index_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value == array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_find_last_zero(|| Ok::<_, TsonicError>(true))
        .is_ok());
    assert!(values
        .try_find_last(|value| Ok::<_, TsonicError>(value == 2))
        .is_ok());
    assert!(values
        .try_find_last_with_index(|value, index| { Ok::<_, TsonicError>(value as f64 > index) })
        .is_ok());
    assert!(values
        .try_find_last_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value == array.len() as i32)
        })
        .is_ok());

    assert!(values
        .try_find_last_index_zero(|| Ok::<_, TsonicError>(true))
        .is_ok());
    assert!(values
        .try_find_last_index(|value| Ok::<_, TsonicError>(value == 2))
        .is_ok());
    assert!(values
        .try_find_last_index_with_index(|value, index| {
            Ok::<_, TsonicError>(value as f64 > index)
        })
        .is_ok());
    assert!(values
        .try_find_last_index_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value == array.len() as i32)
        })
        .is_ok());

    assert!(values.try_some_zero(|| Ok::<_, TsonicError>(false)).is_ok());
    assert!(values
        .try_some(|value| Ok::<_, TsonicError>(value == 2))
        .is_ok());
    assert!(values
        .try_some_with_index(|value, index| Ok::<_, TsonicError>(value as f64 > index))
        .is_ok());
    assert!(values
        .try_some_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value == array.len() as i32)
        })
        .is_ok());
    assert!(values.try_every_zero(|| Ok::<_, TsonicError>(true)).is_ok());
    assert!(values
        .try_every(|value| Ok::<_, TsonicError>(value > 0))
        .is_ok());
    assert!(values
        .try_every_with_index(|value, index| Ok::<_, TsonicError>(value as f64 > index))
        .is_ok());
    assert!(values
        .try_every_with_array(|value, _, array| {
            Ok::<_, TsonicError>(value <= array.len() as i32)
        })
        .is_ok());
}

#[test]
fn fallible_array_callbacks_short_circuit_and_preserve_reduce_errors() {
    let values = JsArray::from_dense(vec![1, 2, 3]);
    let mut visits = Vec::new();
    let failure = values.try_map(|value| {
        visits.push(value);
        if value == 2 {
            Err(JsError::error("stop").into())
        } else {
            Ok(value)
        }
    });
    assert_eq!(
        failure.unwrap_err(),
        TsonicError::Js(JsError::error("stop"))
    );
    assert_eq!(visits, vec![1, 2]);

    let empty = JsArray::<i32>::new();
    let failure = empty
        .try_reduce_from_first(|left, right| Ok::<_, TsonicError>(left + right))
        .unwrap_err();
    assert_eq!(
        failure,
        TsonicError::Js(JsError::new(
            JsErrorKind::TypeError,
            "Reduce of empty array with no initial value"
        ))
    );
}

#[test]
fn fallible_sort_callbacks_are_stable_preserve_holes_and_publish_atomically() {
    let values = JsArray::from_sparse(
        5,
        vec![(0, (2, "first")), (2, (1, "middle")), (3, (2, "second"))],
    );
    let sorted = values
        .try_sort(|left, right| Ok::<_, TsonicError>(f64::from(left.0 - right.0)))
        .unwrap();
    assert!(values.ptr_eq(&sorted));
    assert_eq!(
        values.values(),
        vec![
            Some((1, "middle")),
            Some((2, "first")),
            Some((2, "second")),
            None,
            None,
        ],
    );

    let unchanged = JsArray::from_dense(vec![3, 2, 1]);
    let before = unchanged.values();
    let failure = unchanged.try_sort(|left, right| {
        if left == 2 || right == 2 {
            Err(JsError::error("stop").into())
        } else {
            Ok(f64::from(left - right))
        }
    });
    assert_eq!(
        failure.unwrap_err(),
        TsonicError::Js(JsError::error("stop"))
    );
    assert_eq!(unchanged.values(), before);

    let nan = JsArray::from_dense(vec![2, 1]);
    nan.try_sort(|_, _| Ok::<_, TsonicError>(f64::NAN)).unwrap();
    assert_eq!(nan.values(), vec![Some(2), Some(1)]);

    let unary = JsArray::from_dense(vec![2, 1]);
    unary
        .try_sort_value(|value| Ok::<_, TsonicError>(f64::from(value - 1)))
        .unwrap();
    assert_eq!(unary.len(), 2);

    let zero = JsArray::from_dense(vec![2, 1]);
    zero.try_sort_zero(|| Ok::<_, TsonicError>(0.0)).unwrap();
    assert_eq!(zero.values(), vec![Some(2), Some(1)]);
}

#[test]
fn fallible_vector_factories_preserve_callback_arity_and_short_circuiting() {
    let values = vec![2, 4, 6];
    assert_eq!(
        tsonic_rust_js::array::from_vec_try_map_zero(&values, || Ok::<_, TsonicError>(7))
            .unwrap()
            .values(),
        vec![Some(7), Some(7), Some(7)]
    );
    assert_eq!(
        tsonic_rust_js::array::from_vec_try_map(&values, |value| {
            Ok::<_, TsonicError>(value / 2)
        })
        .unwrap()
        .values(),
        vec![Some(1), Some(2), Some(3)]
    );
    assert_eq!(
        tsonic_rust_js::array::from_vec_try_map_with_index(&values, |value, index| {
            Ok::<_, TsonicError>(value + index as i32)
        })
        .unwrap()
        .values(),
        vec![Some(2), Some(5), Some(8)]
    );

    let mut visits = 0;
    let failure = tsonic_rust_js::array::from_vec_try_map(&values, |value| {
        visits += 1;
        if value == 4 {
            Err(TsonicError::unsupported("stop"))
        } else {
            Ok(value)
        }
    });
    assert_eq!(failure, Err(TsonicError::unsupported("stop")));
    assert_eq!(visits, 2);
}
