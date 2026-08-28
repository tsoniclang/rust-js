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
}
