use tsonic_rust_js::{Int16Array, JsArray, Uint8Array};

#[test]
fn typed_array_get_set_fill_and_slice() {
    let values = Int16Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    assert_eq!(values.length(), 4.0);
    assert_eq!(values.byte_length(), 8.0);
    assert_eq!(values.get_number(2.0), Some(3.0));
    values.set_number(1.0, 9.0);
    values.fill(7.0, -2.0, None);
    assert_eq!(values.get_number(1.0), Some(9.0));
    assert_eq!(values.get_number(2.0), Some(7.0));

    let copy = values.slice(1.0, Some(3.0));
    values.set_number(1.0, 100.0);
    assert_eq!(copy.get_number(0.0), Some(9.0));
}

#[test]
fn typed_array_can_be_created_from_array_buffer() {
    let buffer = tsonic_rust_js::ArrayBuffer::new(4.0).unwrap();
    buffer.as_mut_bytes().copy_from_slice(&[1, 0, 2, 0]);
    let typed = tsonic_rust_js::Uint16Array::from_buffer_only(buffer.clone()).unwrap();
    assert_eq!(typed.length(), 2.0);
    assert_eq!(typed.get_number(0.0), Some(1.0));
    assert_eq!(typed.get_number(1.0), Some(2.0));
    typed.set_number(0.0, 9.0);
    assert_eq!(&*buffer.as_bytes(), &[9, 0, 2, 0]);
}

#[test]
fn typed_array_subarray_is_shared_view() {
    let values = Uint8Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let view = values.subarray(1.0, Some(3.0));
    view.set_number(0.0, 99.0);
    assert_eq!(values.get_number(1.0), Some(99.0));
}

#[test]
fn typed_array_clone_preserves_view_identity_while_subarray_creates_a_new_view() {
    let values = Uint8Array::from_vec(vec![1.0, 2.0, 3.0]).unwrap();
    let alias = values.clone();
    let subarray = values.subarray(0.0, None);

    assert_eq!(values, alias);
    assert_ne!(values, subarray);
    subarray.set_number(1.0, 9.0);
    assert_eq!(values.get_number(1.0), Some(9.0));
}

#[test]
fn typed_array_set_accepts_exact_js_array_sources() {
    let values = Uint8Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    let source = JsArray::from_dense(vec![9.0, 8.0]);
    values.set_from_array(&source, 1.0).unwrap();
    assert_eq!(values.get_number(0.0), Some(1.0));
    assert_eq!(values.get_number(1.0), Some(9.0));
    assert_eq!(values.get_number(2.0), Some(8.0));
    assert!(values
        .set_from_array(&JsArray::from_dense(vec![1.0, 2.0, 3.0]), 3.0)
        .is_err());
    assert!(values.set_from_array(&source, f64::INFINITY).is_err());

    values
        .set_from_array_default(&JsArray::from_dense(vec![7.0, 6.0]))
        .unwrap();
    values.set_from_fixed_array(&[5.0, 4.0], 1.0).unwrap();
    let typed_source = Int16Array::from_vec(vec![3.0, 2.0]).unwrap();
    values.set_from_typed_array(&typed_source, 2.0).unwrap();
    assert_eq!(values.get_number(0.0), Some(7.0));
    assert_eq!(values.get_number(1.0), Some(5.0));
    assert_eq!(values.get_number(2.0), Some(3.0));
    assert_eq!(values.get_number(3.0), Some(2.0));
}

#[test]
fn typed_array_view_constructors_subarrays_and_sorts_are_closed() {
    let buffer = tsonic_rust_js::ArrayBuffer::new(8.0).unwrap();
    let offset = Uint8Array::from_buffer_offset(buffer.clone(), 2.0).unwrap();
    assert_eq!(offset.byte_offset(), 2.0);
    assert_eq!(offset.bytes_per_element(), 1.0);
    let bounded = Uint8Array::from_buffer_length(buffer, 2.0, 4.0).unwrap();
    bounded
        .set_from_fixed_array(&[4.0, 1.0, 3.0, 2.0], 0.0)
        .unwrap();

    assert_eq!(bounded.subarray_all().length(), 4.0);
    assert_eq!(bounded.subarray_from(1.0).length(), 3.0);
    assert_eq!(bounded.subarray_to(1.0, 3.0).length(), 2.0);
    bounded.sort_default();
    assert_eq!(bounded.get_number(0.0), Some(1.0));
    bounded.try_sort_by(|left, right| Ok(right - left)).unwrap();
    assert_eq!(bounded.get_number(0.0), Some(4.0));
}
