use tsonic_rust_js::{Int16Array, JsArray, Uint8Array};

fn verify_native_pair<Source, Target>()
where
    Source: tsonic_rust_js::typed_array::TypedElement + num_traits::AsPrimitive<Target>,
    Target: tsonic_rust_js::typed_array::TypedElement,
{
    use tsonic_rust_js::typed_array::{TypedArray, TypedElement};
    let input = [-40000.9, -257.9, -1.9, 0.0, 1.9, 127.9, 256.9,
        u32::MAX as f64, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    let source = TypedArray::<Source>::from_vec(input.to_vec()).unwrap();
    let mut expected = vec![0; input.len() * Target::BYTES_PER_ELEMENT];
    for (value, output) in input.into_iter().zip(expected.chunks_exact_mut(Target::BYTES_PER_ELEMENT)) {
        let converted: Target = Source::from_number(value).as_();
        converted.write_bytes(output);
    }
    let copy = TypedArray::<Target>::from_typed_array(&source).unwrap();
    assert_eq!(copy.with_bytes(<[u8]>::to_vec), expected);
    let assigned = TypedArray::<Target>::new(input.len() as f64).unwrap();
    assigned.set_from_typed_array(&source, 0.0).unwrap();
    assert_eq!(assigned.with_bytes(<[u8]>::to_vec), expected);
    source.set_number(0.0, 7.0);
    assert_eq!(copy.with_bytes(<[u8]>::to_vec), expected);
    assert_eq!(assigned.with_bytes(<[u8]>::to_vec), expected);
}

#[test]
fn every_native_element_pair_retains_its_source_carrier() {
    use tsonic_rust_js::typed_array::ClampedU8;
    macro_rules! targets {
        ($source:ty) => {{
            verify_native_pair::<$source, i8>();
            verify_native_pair::<$source, u8>();
            verify_native_pair::<$source, ClampedU8>();
            verify_native_pair::<$source, i16>();
            verify_native_pair::<$source, u16>();
            verify_native_pair::<$source, i32>();
            verify_native_pair::<$source, u32>();
            verify_native_pair::<$source, f32>();
            verify_native_pair::<$source, f64>();
        }};
    }
    targets!(i8); targets!(u8); targets!(ClampedU8); targets!(i16); targets!(u16);
    targets!(i32); targets!(u32); targets!(f32); targets!(f64);
}

#[test]
fn native_pair_conversion_preserves_overlapping_and_disjoint_shared_views() {
    use tsonic_rust_js::ArrayBuffer;
    for (source_offset, target_offset) in [(0.0, 2.0), (0.0, 8.0), (8.0, 0.0)] {
        let buffer = ArrayBuffer::new(16.0).unwrap();
        let source = Int16Array::from_buffer(buffer.clone(), source_offset, Some(4.0)).unwrap();
        for index in 0..4 { source.set_number(index as f64, (index + 1) as f64); }
        let target = Uint8Array::from_buffer(buffer.clone(), target_offset, Some(4.0)).unwrap();
        target.set_from_typed_array(&source, 0.0).unwrap();
        assert_eq!(target.with_bytes(<[u8]>::to_vec), [1, 2, 3, 4]);
    }
}

#[test]
fn typed_array_owned_bytes_keep_exact_native_length_and_view_identity() {
    use tsonic_rust_runtime::ObjectIdentityCarrier;

    let bytes = Uint8Array::from_bytes(vec![3, 4, 5]);
    let alias = bytes.clone();
    assert_eq!(bytes.len(), 3);
    assert!(!bytes.is_empty());
    assert_eq!(alias.object_identity().key(), bytes.object_identity().key());
    alias.set_number(0.0, 7.0);
    assert_eq!(bytes.get_number(0.0), Some(7.0));
    assert!(Uint8Array::from_bytes(Vec::new()).is_empty());
}

#[test]
fn typed_array_byte_reader_respects_the_selected_view() {
    let values = Int16Array::from_vec(vec![0x1234 as f64, 0x5678 as f64]).unwrap();
    let view = values.subarray(1.0, None);
    assert_eq!(view.with_bytes(|bytes| bytes.to_vec()), [0x78, 0x56]);
    values.set_number(1.0, 0.0);
    assert_eq!(view.with_bytes(|bytes| bytes.to_vec()), [0, 0]);
    assert_eq!(values.subarray(2.0, None).with_bytes(<[u8]>::len), 0);
}

#[test]
fn typed_array_copy_converts_values_without_aliasing_the_original_view() {
    let original = Int16Array::from_vec(vec![0.0, -1.0, 257.0, 3.0]).unwrap();
    let view = original.subarray(1.0, Some(3.0));
    let copy = Uint8Array::from_typed_array(&view).unwrap();
    assert_eq!(copy.with_bytes(|bytes| bytes.to_vec()), vec![255, 1]);
    view.set_number(0.0, 9.0);
    assert_eq!(copy.get_number(0.0), Some(255.0));
    copy.set_number(1.0, 7.0);
    assert_eq!(original.get_number(2.0), Some(257.0));
    assert!(Uint8Array::from_typed_array(&original.subarray(4.0, None))
        .unwrap()
        .is_empty());
}

#[test]
fn typed_array_mutable_bytes_only_expose_the_selected_backing_window() {
    let original = Uint8Array::from_bytes(vec![1, 2, 3, 4]);
    let view = original.subarray(1.0, Some(3.0));
    let written = view.with_mut_bytes(|bytes| {
        bytes.copy_from_slice(&[8, 9]);
        bytes.len()
    });
    assert_eq!(written, 2);
    assert_eq!(
        original.with_bytes(|bytes| bytes.to_vec()),
        vec![1, 8, 9, 4]
    );
    assert_eq!(view.with_bytes(|bytes| bytes.to_vec()), vec![8, 9]);
    assert_eq!(
        original
            .subarray(4.0, None)
            .with_mut_bytes(|bytes| bytes.len()),
        0
    );
}

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
