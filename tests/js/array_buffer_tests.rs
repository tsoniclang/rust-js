use tsonic_rust_js::ArrayBuffer;

#[test]
fn array_buffer_slice_copies() {
    let buffer = ArrayBuffer::from_bytes(vec![1, 2, 3, 4]);
    let slice = buffer.slice(1, Some(3));
    assert_eq!(&*slice.as_bytes(), &[2, 3]);
    assert_eq!(buffer.byte_length(), 4);
}

#[test]
fn array_buffer_slice_with_reversed_bounds_is_empty() {
    let buffer = ArrayBuffer::from_bytes(vec![1, 2, 3, 4]);
    assert!(buffer.slice(3, Some(1)).as_bytes().is_empty());
}

#[test]
fn array_buffer_clones_preserve_identity_but_equal_bytes_do_not_define_identity() {
    let buffer = ArrayBuffer::from_bytes(vec![1, 2, 3]);
    let alias = buffer.clone();
    let distinct = ArrayBuffer::from_bytes(vec![1, 2, 3]);

    assert_eq!(buffer, alias);
    assert_ne!(buffer, distinct);
    alias.as_mut_bytes()[0] = 9;
    assert_eq!(buffer.as_bytes()[0], 9);
}
