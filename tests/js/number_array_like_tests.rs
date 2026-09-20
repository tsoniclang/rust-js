use tsonic_rust_js::abi::{number_array_from, number_array_get, number_array_length};
use tsonic_rust_js::{Float64Array, Int16Array, JsArray, Uint8Array};

#[test]
fn number_array_access_preserves_the_selected_storage_and_index_rules() {
    let values = JsArray::from_dense(vec![1.0, 2.0]);
    let alias = values.clone();
    alias.set_number(-1.0, 19.0);
    assert_eq!(number_array_get(&values, -1.0), Some(19.0));
    alias.set(0, 7.0);
    assert_eq!(number_array_get(&values, 0.0), Some(7.0));
    assert_eq!(number_array_length(&values), 2.0);
    assert_eq!(number_array_get(&values, 0.5), None);
    assert_eq!(number_array_get(&values, 2.0), None);
    let bytes = Uint8Array::from_bytes(vec![3, 4, 5]);
    let view = bytes.subarray(1.0, None);
    bytes.set_number(1.0, 9.0);
    assert_eq!(number_array_get(&view, 0.0), Some(9.0));
    assert_eq!(number_array_length(&view), 2.0);
    for index in [-1.0, 0.5, 2.0, f64::NAN, f64::INFINITY] {
        assert_eq!(number_array_get(&view, index), None);
    }
}

#[test]
fn number_array_copies_preserve_values_widths_and_output_independence() {
    let values = Int16Array::from_vec(vec![-32768.0, 32767.0]).unwrap();
    let copy = number_array_from(&values);
    assert_eq!(copy.get(0), Some(-32768.0));
    assert_eq!(copy.get(1), Some(32767.0));
    values.set_number(0.0, 5.0);
    assert_eq!(copy.get(0), Some(-32768.0));
    copy.set(1, 7.0);
    assert_eq!(values.get_number(1.0), Some(32767.0));
    let floats = Float64Array::from_vec(vec![-0.0, f64::NAN, f64::INFINITY]).unwrap();
    let copy = number_array_from(&floats);
    assert!(copy.get(0).unwrap().is_sign_negative());
    assert!(copy.get(1).unwrap().is_nan());
    assert_eq!(copy.get(2), Some(f64::INFINITY));
    assert_eq!(
        number_array_length(&number_array_from(&Uint8Array::from_bytes(Vec::new()))),
        0.0
    );
}

#[test]
fn number_array_copy_preserves_initialized_native_defaults() {
    let values = JsArray::<f64>::with_length(2);
    values.set(0, 3.0);
    assert_eq!(number_array_get(&values, 1.0), Some(0.0));
    assert_eq!(number_array_from(&values).values(), vec![3.0, 0.0]);
}
