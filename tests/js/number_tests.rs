use tsonic_rust_js::number;

#[test]
fn parse_integer_uses_the_native_integer_domain_and_full_token() {
    use num_bigint::BigInt;
    for radix in 2_u32..=36 {
        for value in [i128::MIN, -9_007_199_254_740_993, -1, 0, 1, 9_007_199_254_740_993, i128::MAX] {
            let digits = BigInt::from(value).to_str_radix(radix);
            let expected = i128::from_str_radix(&digits, radix).unwrap() as f64;
            assert_eq!(number::parse_int(&digits, Some(radix as f64)).to_bits(), expected.to_bits());
        }
    }
    for text in ["", " 42", "42 ", "1e2", "0x10", "123abc", "1e+", "-", "\u{feff}42",
        "170141183460469231731687303715884105728", "-170141183460469231731687303715884105729"] {
        assert!(number::parse_int(text, None).is_nan(), "{text}");
    }
    for radix in [0.0, 1.0, 37.0, 2.5, 4_294_967_298.0, f64::NAN, f64::INFINITY] {
        assert!(number::parse_int("10", Some(radix)).is_nan());
    }
    assert_eq!(number::parse_int("-0", None).to_bits(), 0_f64.to_bits());
}

#[test]
fn parse_float_delegates_to_rust_from_str() {
    for text in ["0", "-0", "3.25", "-3.25e1", "1e400", "NaN", "inf", "-inf", "Infinity",
        "+Infinity", " 1.5", "1.5 ", "1.5x", "0x10", "1e", "1e+", "", "-", "+"] {
        let actual = number::parse_float(text);
        match text.parse::<f64>() {
            Ok(expected) if !expected.is_nan() => assert_eq!(actual.to_bits(), expected.to_bits(), "{text}"),
            _ => assert!(actual.is_nan(), "{text}"),
        }
    }
}

#[test]
fn number_constants_are_exposed() {
    let max = std::hint::black_box(number::MAX_VALUE);
    let min = std::hint::black_box(number::MIN_VALUE);
    let max_safe = std::hint::black_box(number::MAX_SAFE_INTEGER);
    let min_safe = std::hint::black_box(number::MIN_SAFE_INTEGER);
    let positive_infinity = std::hint::black_box(number::POSITIVE_INFINITY);
    let negative_infinity = std::hint::black_box(number::NEGATIVE_INFINITY);
    let nan = std::hint::black_box(number::NAN);
    let epsilon = std::hint::black_box(number::EPSILON);
    assert_eq!(max, f64::MAX);
    assert_eq!(min.to_bits(), 1);
    assert_eq!(max_safe, 9_007_199_254_740_991.0);
    assert_eq!(min_safe, -9_007_199_254_740_991.0);
    assert_eq!(positive_infinity, f64::INFINITY);
    assert_eq!(negative_infinity, f64::NEG_INFINITY);
    assert!(nan.is_nan());
    assert!(epsilon > 0.0);
}


#[test]
fn floating_text_uses_each_native_width_and_formatter() {
    for value in [-0.0_f64, 0.0, 1e-7, 1e21, 12.5, 2.55, f64::MIN_POSITIVE,
        f64::from_bits(1), f64::MAX, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(number::to_string(value), value.to_string());
        assert_eq!(number::to_precision_default(value), value.to_string());
        assert_eq!(number::to_fixed_default(value), format!("{value:.0}"));
        assert_eq!(number::to_exponential_default(value), format!("{value:e}"));
        for digits in [0_usize, 1, 2, 100, 101] {
            assert_eq!(number::to_fixed_digits(value, digits as f64).unwrap(), format!("{value:.digits$}"));
            assert_eq!(number::to_exponential_digits(value, digits as f64).unwrap(), format!("{value:.digits$e}"));
            assert_eq!(number::to_precision_digits(value, (digits + 1) as f64).unwrap(), format!("{value:.digits$e}"));
        }
    }
    for value in [0.1_f32, f32::MIN_POSITIVE, f32::MAX, -0.0] {
        assert_eq!(number::to_string(value), value.to_string());
        assert_eq!(number::to_fixed_digits(value, 3.0).unwrap(), format!("{value:.3}"));
        assert_eq!(number::to_exponential_default(value), format!("{value:e}"));
    }
}

#[test]
fn integer_text_never_passes_through_floating_point() {
    use num_bigint::{BigInt, BigUint};
    for value in [i128::MIN, -9_007_199_254_740_993, 0, 9_007_199_254_740_993, i128::MAX] {
        assert_eq!(number::to_string(value), value.to_string());
        assert_eq!(number::to_fixed_default(value), value.to_string());
        assert_eq!(number::to_fixed_digits(value, 2.0).unwrap(), format!("{value}.00"));
        assert_eq!(number::to_exponential_default(value), format!("{value:e}"));
        for radix in 2_u32..=36 {
            assert_eq!(number::to_string_radix(value, radix as f64).unwrap(), BigInt::from(value).to_str_radix(radix));
        }
    }
    for value in [0_u128, 9_007_199_254_740_993, u128::MAX] {
        assert_eq!(number::to_string(value), value.to_string());
        for radix in 2_u32..=36 {
            assert_eq!(number::to_string_radix(value, radix as f64).unwrap(), BigUint::from(value).to_str_radix(radix));
        }
    }
    assert_eq!(number::value_of(i128::MAX), i128::MAX);
    assert_eq!(number::value_of(u128::MAX), u128::MAX);
}

#[test]
fn formatting_rejects_invalid_counts_without_coercion_or_large_allocations() {
    for count in [-1.0, 0.5, f64::NAN, f64::INFINITY, usize::MAX as f64] {
        assert!(number::to_fixed_digits(1.0, count).is_err());
        assert!(number::to_exponential_digits(1.0, count).is_err());
        assert!(number::to_precision_digits(1.0, count).is_err());
    }
    assert!(number::to_precision_digits(1.0, 0.0).is_err());
    for radix in [0.0, 1.0, 2.5, 37.0, f64::NAN, f64::INFINITY] {
        assert!(number::to_string_radix(42_i32, radix).is_err());
    }
}

#[test]
fn same_value_zero_and_strict_equal_for_nans_and_zeroes() {
    use tsonic_rust_js::equality::{same_value_zero_f64, strict_equal_f64};
    assert!(same_value_zero_f64(f64::NAN, f64::NAN));
    assert!(!strict_equal_f64(f64::NAN, f64::NAN));
    assert!(same_value_zero_f64(0.0, -0.0));
    assert!(strict_equal_f64(0.0, -0.0));
}
