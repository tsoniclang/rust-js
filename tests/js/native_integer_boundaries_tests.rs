use tsonic_rust_js::{abi, array::ArrayLength, bigint, JsErrorKind};

#[test]
fn split_limits_belong_to_the_explicit_api_not_the_storage_domain() {
    use tsonic_rust_js::{exact_string, regexp::JsRegExp, string, JsString};
    let input = JsString::from_utf8("a,b,c");
    let separator = JsString::from_utf8(",");
    let expression = JsRegExp::new(separator.clone(), JsString::new()).unwrap();
    for (limit, length) in [
        (-1.0, 3),
        (1.5, 1),
        (f64::NAN, 0),
        (f64::INFINITY, 0),
        (4_294_967_296.0, 0),
        (4_294_967_297.0, 1),
    ] {
        assert_eq!(string::split("a,b,c", ",", limit).unwrap().len(), length);
        assert_eq!(exact_string::split(&input, &separator, limit).len(), length);
        assert_eq!(expression.split(&input, Some(limit)).unwrap().len(), length);
    }
}

#[test]
fn explicit_integer_api_conversion_matches_modulo_bits() {
    use tsonic_rust_js::{math, typed_array::TypedElement};
    for exponent in 0..=2047_u64 {
        for fraction in [0, 1, 0x876543210, (1_u64 << 52) - 1] {
            for sign in [0, 1_u64 << 63] {
                let value = f64::from_bits(sign | (exponent << 52) | fraction);
                let remainder = if value.is_finite() {
                    value.trunc() % 4_294_967_296.0
                } else {
                    0.0
                };
                let expected = if remainder < 0.0 {
                    remainder + 4_294_967_296.0
                } else {
                    remainder
                } as u32;
                assert_eq!(u32::from_number(value), expected, "{value}");
                assert_eq!(i8::from_number(value), expected as i8);
                assert_eq!(math::imul(value, 1_i32), expected as i32);
                assert_eq!(math::clz32(value), expected.leading_zeros() as i32);
            }
        }
    }
    assert_eq!(math::imul(9_007_199_254_740_993_i64, 1_i32), 1);
}

#[test]
fn safe_integer_query_does_not_limit_native_integer_storage() {
    use tsonic_rust_js::number;
    assert!(!number::is_safe_integer(9_007_199_254_740_993_i64));
    assert!(number::is_integer(9_007_199_254_740_993_i64));
    assert!(number::is_safe_integer(16_777_216_f32));
    assert!(!number::is_safe_integer(9_007_199_254_740_992_f32));
    assert_eq!(
        number::to_string(9_007_199_254_740_993_i64),
        "9007199254740993"
    );
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
