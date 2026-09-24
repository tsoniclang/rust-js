use tsonic_rust_js::{ArrayBuffer, DataView, Float32Array, Uint32Array, Uint8Array};

#[test]
fn ordinary_numeric_arrays_preserve_exact_native_elements() {
    use tsonic_rust_js::{array::JsArray, Uint8ClampedArray};
    let values = JsArray::from_dense(vec![9_007_199_254_740_993_u64, u64::MAX]);
    let bytes = Uint8Array::from_array(&values).unwrap();
    assert_eq!(bytes.get_number(0_usize), Some(1));
    assert_eq!(bytes.get_number(1_usize), Some(255));
    assert_eq!(
        Uint32Array::from_array(&values)
            .unwrap()
            .get_number(1_usize),
        Some(u32::MAX)
    );
    assert_eq!(
        Uint8ClampedArray::from_array(&values)
            .unwrap()
            .get_number(0_usize),
        Some(255)
    );
    assert!(bytes.set_from_array(&values, 1_usize).is_err());
    assert_eq!(bytes.get_number(0_usize), Some(1));
    bytes
        .set_from_fixed_array(&[7_u64, 8_u64], 0_usize)
        .unwrap();
    assert_eq!(
        values.with_values(|values| values[0]),
        9_007_199_254_740_993
    );
    bytes.set_from_array_default(&values).unwrap();
    assert_eq!(bytes.get_number(0_usize), Some(1));
}

#[test]
fn native_indices_preserve_views_and_reject_unaddressable_offsets() {
    let buffer = ArrayBuffer::new(16_usize).unwrap();
    let view = DataView::from_buffer_length(buffer.clone(), 4_u64, 8_u32).unwrap();
    view.set_uint32(0_usize, 17_u32, true).unwrap();
    let words = Uint32Array::from_buffer_length(buffer.clone(), 4_u32, 2_usize).unwrap();
    assert_eq!(words.get_number(0_u64), Some(17));
    let alias = words.subarray_to(0_i64, 1_usize);
    alias.set_number(0_usize, 29_u32);
    assert_eq!(view.get_uint32(0_u64, true).unwrap(), 29);
    let copied = words.slice_to(0_usize, 1_u64);
    copied.set_number(0_u64, 37_u32);
    assert_eq!(words.at(-2_i64), Some(29));
    assert_eq!(words.at(-0.5_f64), Some(29));
    assert_eq!(words.get_number(usize::MAX), None);
    assert_eq!(words.at(i128::MIN), None);
    assert_eq!(words.subarray_to(i128::MIN, u128::MAX).length(), 2);
    assert_eq!(buffer.slice_to(usize::MAX, u128::MAX).byte_length(), 0);
    assert!(view.get_uint32(usize::MAX, true).is_err());
    assert!(DataView::from_buffer_offset(buffer.clone(), u64::MAX).is_err());
    assert!(Uint32Array::from_buffer_offset(buffer.clone(), 1_u32).is_err());
    assert!(Uint32Array::from_buffer_length(buffer, 0_usize, usize::MAX).is_err());
}

#[test]
fn binary_writes_keep_exact_native_integer_bits() {
    let view = DataView::from_buffer(ArrayBuffer::new(8.0).unwrap()).unwrap();
    view.set_uint32(0.0, 9007199254740993_i64, true).unwrap();
    assert_eq!(view.get_uint32(0.0, true).unwrap(), 1);
    view.set_int32(0.0, u64::MAX, false).unwrap();
    assert_eq!(view.get_int32(0.0, false).unwrap(), -1);
    view.set_uint16(0.0, u128::MAX, true).unwrap();
    assert_eq!(view.get_uint16(0.0, true).unwrap(), u16::MAX);
    view.set_int16(0.0, i128::MIN + 1, false).unwrap();
    assert_eq!(view.get_int16(0.0, false).unwrap(), 1);
    view.set_int8(0.0, i64::MIN + 255).unwrap();
    assert_eq!(view.get_int8(0.0).unwrap(), -1);
    view.set_uint8(0.0, u64::MAX).unwrap();
    assert_eq!(view.get_uint8(0.0).unwrap(), u8::MAX);
    view.set_uint32(0.0, -1.5, false).unwrap();
    assert_eq!(view.get_uint32(0.0, false).unwrap(), u32::MAX);
}

#[test]
fn typed_writes_and_fill_keep_native_integers_and_clamping() {
    let words = Uint32Array::new(2.0).unwrap();
    words.set_number(0.0, 9007199254740993_i64);
    assert_eq!(words.get_number(0.0), Some(1));
    words.fill_from(u128::MAX, 1.0);
    assert_eq!(words.get_number(1.0), Some(u32::MAX));
    words.fill_all(-1.5);
    assert_eq!(words.get_number(0.0), Some(u32::MAX));
    let clamped = tsonic_rust_js::Uint8ClampedArray::new(2.0).unwrap();
    clamped.set_number(0.0, u128::MAX);
    clamped.set_number(1.0, i128::MIN);
    assert_eq!(clamped.get_number(0.0), Some(u8::MAX));
    assert_eq!(clamped.get_number(1.0), Some(0));
    clamped.fill_all(2.5);
    assert_eq!(clamped.get_number(0.0), Some(2));
    words.set_number(-1.0, 17_i64);
    assert_eq!(words.get_number(0.0), Some(u32::MAX));
}

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
    values.sort_by(|left: u32, right: u32| {
        if left < right {
            -1.0
        } else if left > right {
            1.0
        } else {
            0.0
        }
    });
    assert_eq!(values.get_number(0.0), Some(0));
    assert_eq!(values.get_number(2.0), Some(u32::MAX));
}

#[test]
fn closed_native_integer_construction_never_uses_the_floating_variant() {
    use tsonic_rust_js::JsValue;
    macro_rules! check_signed {
        ($($source:ty),+ $(,)?) => {
            $(for value in [<$source>::MIN, 0, <$source>::MAX] {
                assert!(matches!(JsValue::from(value), JsValue::Integer(actual) if actual == i64::from(value)));
            })+
        };
    }
    macro_rules! check_unsigned {
        ($($source:ty),+ $(,)?) => {
            $(for value in [0, <$source>::MAX] {
                assert!(matches!(JsValue::from(value), JsValue::UnsignedInteger(actual) if actual == u64::from(value)));
            })+
        };
    }
    check_signed!(i8, i16, i32, i64);
    check_unsigned!(u8, u16, u32, u64);
    assert!(
        matches!(JsValue::from(isize::MIN), JsValue::Integer(actual) if actual == isize::MIN as i64)
    );
    assert!(
        matches!(JsValue::from(usize::MAX), JsValue::UnsignedInteger(actual) if actual == usize::MAX as u64)
    );
    assert!(matches!(JsValue::from(1.5_f32), JsValue::Number(1.5)));
    assert!(matches!(JsValue::from(1.5_f64), JsValue::Number(1.5)));
}

#[test]
fn closed_native_integers_remain_exact_without_growing_the_value_carrier() {
    use tsonic_rust_js::{
        equality::{JsHash, JsSameValue, JsSameValueZero},
        JsValue,
    };
    if usize::BITS == 64 {
        assert_eq!(std::mem::size_of::<JsValue>(), 40);
        assert_eq!(std::mem::align_of::<JsValue>(), 8);
    }
    for value in [
        i64::MIN,
        -9_007_199_254_740_993,
        -1,
        0,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        let boxed = JsValue::from(value);
        assert!(matches!(boxed, JsValue::Integer(stored) if stored == value));
        assert_eq!(boxed, boxed.clone());
        assert_eq!(boxed.inspect(), value.to_string());
        assert_eq!(
            tsonic_rust_js::json::stringify(&boxed).unwrap(),
            Some(value.to_string())
        );
    }
    for value in [0_u64, 9_007_199_254_740_993, u64::MAX] {
        let boxed = JsValue::from(value);
        assert!(matches!(boxed, JsValue::UnsignedInteger(stored) if stored == value));
        assert_eq!(boxed.inspect(), value.to_string());
        assert_eq!(
            tsonic_rust_js::json::stringify(&boxed).unwrap(),
            Some(value.to_string())
        );
    }
    for value in [-1_i64, 0, 1, 9_007_199_254_740_992] {
        let native = JsValue::from(value);
        let floating = JsValue::Number(value as f64);
        assert_eq!(native, floating);
        assert_eq!(native.js_hash(), floating.js_hash());
        if value >= 0 {
            let unsigned = JsValue::from(value as u64);
            assert_eq!(unsigned, native);
            assert_eq!(unsigned.js_hash(), native.js_hash());
        }
    }
    let exact = JsValue::from(9_007_199_254_740_993_u64);
    assert_ne!(exact, JsValue::from(9_007_199_254_740_992_u64));
    assert_ne!(exact, JsValue::Number(9_007_199_254_740_993_u64 as f64));
    assert_ne!(JsValue::from(u64::MAX), JsValue::Number(u64::MAX as f64));
    assert_ne!(JsValue::from(i64::MAX), JsValue::Number(i64::MAX as f64));
    assert_ne!(JsValue::from(-1_i64), JsValue::from(u64::MAX));
    assert!(!JsValue::from(0_i64).same_value(&JsValue::Number(-0.0)));
    assert!(JsValue::from(0_i64).same_value_zero(&JsValue::Number(-0.0)));
    assert_eq!(
        JsValue::from(0_i64).js_hash(),
        JsValue::Number(-0.0).js_hash()
    );
}
