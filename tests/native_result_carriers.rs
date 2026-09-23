use tsonic_rust_js::{ArrayBuffer, DataView, Float32Array, Uint8Array, Uint32Array};

#[test]
fn binary_results_keep_native_storage_types() {
    let buffer = ArrayBuffer::new(8.0).unwrap();
    let length: usize = buffer.byte_length();
    assert_eq!(length, 8);
    let view = DataView::from_buffer(buffer).unwrap();
    view.set_uint32(0.0, u32::MAX as f64, true).unwrap();
    let word: u32 = view.get_uint32(0.0, true).unwrap();
    assert_eq!(word, u32::MAX);
    let bytes = Uint8Array::new(2.0).unwrap();
    bytes.set_number(0.0, 255.0);
    let byte: Option<u8> = bytes.get_number(0.0);
    let count: usize = bytes.length();
    let position: isize = bytes.index_of_from_start(255.0);
    assert_eq!((byte, count, position), (Some(255), 2, 0));
    assert_eq!(bytes.get_number(f64::NAN), None);
    assert_eq!(bytes.get_number(0.5), None);
    assert_eq!(bytes.get_number(-1.0), None);
    assert_eq!(bytes.get_number(2.0), None);
    let words = Uint32Array::new(1.0).unwrap();
    words.set_number(0.0, u32::MAX as f64);
    let selected: Option<u32> = words.at(0.0);
    assert_eq!(selected, Some(u32::MAX));
    let singles = Float32Array::new(1.0).unwrap();
    singles.set_number(0.0, 1.5);
    let single: Option<f32> = singles.at(0.0);
    assert_eq!(single, Some(1.5));
}

#[test]
fn typed_comparators_receive_native_elements() {
    let values = Uint32Array::from_numbers([u32::MAX as f64, 7.0, 0.0]).unwrap();
    values.sort_by(|left: u32, right: u32| if left < right { -1.0 } else if left > right { 1.0 } else { 0.0 });
    assert_eq!(values.get_number(0.0), Some(0));
    assert_eq!(values.get_number(2.0), Some(u32::MAX));
}
