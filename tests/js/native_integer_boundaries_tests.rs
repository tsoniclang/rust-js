use tsonic_rust_js::{abi, array::ArrayLength, bigint, JsErrorKind};

#[test]
fn native_truncation_preserves_fixed_width_results() {
    assert_eq!(bigint::as_int_native(64.0, &9_007_199_254_740_993_i64).unwrap(), 9_007_199_254_740_993);
    assert_eq!(bigint::as_uint_native(64.0, &-1_i64).unwrap(), u64::MAX as u128);
    assert_eq!(bigint::as_int_native(128.0, &(1_u128 << 127)).unwrap(), i128::MIN);
    assert_eq!(bigint::as_uint_native(128.0, &-1_i128).unwrap(), u128::MAX);
    for bits in [129.0, -1.0, 1.5, f64::NAN, f64::INFINITY] {
        assert_eq!(bigint::as_int_native(bits, &0_u64).unwrap_err().kind(), JsErrorKind::RangeError);
        assert_eq!(bigint::as_uint_native(bits, &0_u64).unwrap_err().kind(), JsErrorKind::RangeError);
    }
}

#[test]
fn array_lengths_follow_usize_without_allocating() {
    assert_eq!(usize::MAX.array_length().unwrap(), usize::MAX);
    assert_eq!((usize::MAX as u128).array_length().unwrap(), usize::MAX);
    assert!((usize::MAX as u128 + 1).array_length().is_err());
    assert!((-1_i64).array_length().is_err());
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.5, (usize::MAX as u128 + 1) as f64] {
        assert!(value.array_length().is_err());
    }
    if usize::BITS == 64 {
        assert_eq!(9_007_199_254_740_993_u64.array_length().unwrap() as u64, 9_007_199_254_740_993);
        assert_eq!(9_007_199_254_740_992_f64.array_length().unwrap() as u64, 9_007_199_254_740_992);
    }
}

#[test]
fn large_width_identity_does_not_allocate_the_width() {
    let value = abi::bigint_from_integer(9_007_199_254_740_993_u64);
    assert_eq!(abi::bigint_as_int_n(9_007_199_254_740_992.0, &value).unwrap(), value);
    assert_eq!(abi::bigint_as_uint_n(9_007_199_254_740_992.0, &value).unwrap(), value);
}
