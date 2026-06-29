use tsonic_rust_js::typed_array::{byte_length, len, Int16Array, Uint8Array};

#[test]
fn typed_array_get_set_fill_and_slice() {
    let mut values = Int16Array::from_vec(vec![1, 2, 3, 4]);
    assert_eq!(len(&values), 4);
    assert_eq!(byte_length(&values), 8);
    assert_eq!(values.get(2), Some(3));
    values.set_index(1, 9);
    values.fill(7, -2, None);
    assert_eq!(values.get(1), Some(9));
    assert_eq!(values.get(2), Some(7));

    let copy = values.slice(1, Some(3));
    values.set_index(1, 100);
    assert_eq!(copy.get(0), Some(9));
}

#[test]
fn typed_array_can_be_created_from_array_buffer() {
    let mut buffer = tsonic_rust_js::ArrayBuffer::new(4);
    buffer.as_mut_bytes().copy_from_slice(&[1, 0, 2, 0]);
    let typed = tsonic_rust_js::Uint16Array::from_buffer(buffer);
    assert_eq!(typed.len(), 2);
    assert_eq!(typed.get(0), Some(1));
    assert_eq!(typed.get(1), Some(2));
}

#[test]
fn typed_array_subarray_is_shared_view() {
    let values = Uint8Array::from_vec(vec![1, 2, 3, 4]);
    let mut view = values.subarray(1, Some(3));
    view.set_index(0, 99);
    assert_eq!(values.get(1), Some(99));
}

#[test]
fn typed_array_set_source_and_map_helpers() {
    let mut values = Uint8Array::from_vec(vec![1, 2, 3, 4]);
    values.set_from_slice(&[9, 8], 1).unwrap();
    assert_eq!(values.get(0), Some(1));
    assert_eq!(values.get(1), Some(9));
    assert_eq!(values.get(2), Some(8));
    assert!(values.set_from_slice(&[1, 2, 3], 3).is_err());
    assert_eq!(values.map(|value| value + 1), vec![2, 10, 9, 5]);
}
