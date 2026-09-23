use tsonic_rust_js::{abi, array::ArrayLength, bigint, JsErrorKind};

#[test]
fn split_limits_are_native_sizes_not_modulo_words() {
    use tsonic_rust_js::{exact_string, regexp::JsRegExp, string, JsString};
    let input = JsString::from_utf8("a,b,c");
    let separator = JsString::from_utf8(",");
    let expression = JsRegExp::new(separator.clone(), JsString::new()).unwrap();
    for invalid in [-1.0, 1.5, f64::NAN, f64::INFINITY] {
        assert!(string::split("a,b,c", ",", invalid).is_err());
        assert!(exact_string::split(&input, &separator, invalid).is_err());
        assert!(expression.split(&input, Some(invalid)).is_err());
    }
    if usize::BITS == 64 {
        let limit = 4_294_967_296.0;
        assert_eq!(string::split("a,b,c", ",", limit).unwrap().len(), 3);
        assert_eq!(
            exact_string::split(&input, &separator, limit)
                .unwrap()
                .len(),
            3
        );
        assert_eq!(expression.split(&input, Some(limit)).unwrap().len(), 3);
    }
}

#[test]
fn typed_elements_use_native_rust_float_casts() {
    use tsonic_rust_js::typed_array::TypedElement;
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        1.9,
        256.0,
        4_294_967_296.0,
    ] {
        assert_eq!(i8::from_number(value), value as i8);
        assert_eq!(u8::from_number(value), value as u8);
        assert_eq!(i16::from_number(value), value as i16);
        assert_eq!(u16::from_number(value), value as u16);
        assert_eq!(i32::from_number(value), value as i32);
        assert_eq!(u32::from_number(value), value as u32);
    }
}

#[test]
fn native_truncation_preserves_fixed_width_results() {
    assert_eq!(
        bigint::as_int_native(64.0, &9_007_199_254_740_993_i64).unwrap(),
        9_007_199_254_740_993
    );
    assert_eq!(
        bigint::as_uint_native(64.0, &-1_i64).unwrap(),
        u64::MAX as u128
    );
    assert_eq!(
        bigint::as_int_native(128.0, &(1_u128 << 127)).unwrap(),
        i128::MIN
    );
    assert_eq!(bigint::as_uint_native(128.0, &-1_i128).unwrap(), u128::MAX);
    for bits in [129.0, -1.0, 1.5, f64::NAN, f64::INFINITY] {
        assert_eq!(
            bigint::as_int_native(bits, &0_u64).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
        assert_eq!(
            bigint::as_uint_native(bits, &0_u64).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
    }
}

#[test]
fn array_lengths_follow_usize_without_allocating() {
    assert_eq!(usize::MAX.array_length().unwrap(), usize::MAX);
    assert_eq!((usize::MAX as u128).array_length().unwrap(), usize::MAX);
    assert!((usize::MAX as u128 + 1).array_length().is_err());
    assert!((-1_i64).array_length().is_err());
    for value in [
        f64::NAN,
        f64::INFINITY,
        -1.0,
        0.5,
        (usize::MAX as u128 + 1) as f64,
    ] {
        assert!(value.array_length().is_err());
    }
    if usize::BITS == 64 {
        assert_eq!(
            9_007_199_254_740_993_u64.array_length().unwrap() as u64,
            9_007_199_254_740_993
        );
        assert_eq!(
            9_007_199_254_740_992_f64.array_length().unwrap() as u64,
            9_007_199_254_740_992
        );
    }
}

#[test]
fn large_width_identity_does_not_allocate_the_width() {
    let value = abi::bigint_from_integer(9_007_199_254_740_993_u64);
    assert_eq!(
        abi::bigint_as_int_n(9_007_199_254_740_992.0, &value).unwrap(),
        value
    );
    assert_eq!(
        abi::bigint_as_uint_n(9_007_199_254_740_992.0, &value).unwrap(),
        value
    );
}
