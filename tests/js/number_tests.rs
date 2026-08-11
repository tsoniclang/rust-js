use tsonic_rust_js::equality::{same_value_zero_f64, strict_equal_f64};
use tsonic_rust_js::number;

#[test]
fn parse_int_radix_examples() {
    assert_eq!(number::parse_int("ff", Some(16.0)), 255.0);
    assert!(number::parse_int("08", Some(10.0)).is_finite());
    assert!(number::parse_int("08", None).is_finite());
    assert_eq!(number::parse_int("08", None), 8.0);
    assert!(number::parse_int("xyz", None).is_nan());
    assert!(number::parse_int("2", Some(1.0)).is_nan());
    assert_eq!(number::parse_int("0x10", Some(10.0)), 0.0);
    assert_eq!(number::parse_int("0x10", None), 16.0);
    assert_eq!(number::parse_int("10", Some(4_294_967_298.0)), 2.0);
    assert!(number::parse_int("-0", None).is_sign_negative());
    assert_eq!(number::parse_int("\u{feff}42", None), 42.0,);
    assert!(number::parse_int("\u{85}42", None).is_nan());
    assert!(number::parse_int(&"1".repeat(309), Some(10.0)).is_finite());
}

#[test]
fn parse_float_prefix_parse() {
    assert_eq!(number::parse_float("  +1.5x"), 1.5);
    assert_eq!(number::parse_float("  -1.5e+2"), -150.0);
    assert_eq!(number::parse_float("1.5x"), 1.5);
    assert_eq!(number::parse_float("0x10"), 0.0);
    assert_eq!(number::parse_float("-3.25e1"), -32.5);
    assert_eq!(number::parse_float("1e"), 1.0);
    assert_eq!(number::parse_float("1e+"), 1.0);
    assert_eq!(number::parse_float("Infinityx"), f64::INFINITY);
    assert!(number::parse_float("infinityx").is_nan());
    assert!(number::parse_float("x").is_nan());
    assert!(
        number::parse_float("Infinity").is_infinite()
            && number::parse_float("Infinity").is_sign_positive()
    );
    assert!(
        number::parse_float("+Infinity").is_infinite()
            && number::parse_float("+Infinity").is_sign_positive()
    );
    assert!(
        number::parse_float("-Infinity").is_infinite()
            && number::parse_float("-Infinity").is_sign_negative()
    );
}

#[test]
fn number_predicates() {
    assert!(number::is_nan(f64::NAN));
    assert!(!number::is_nan(1.2));
    assert!(number::is_finite(0.0));
    assert!(!number::is_finite(f64::INFINITY));
    assert!(number::is_integer(42.0));
    assert!(!number::is_integer(42.5));
    assert!(!number::is_safe_integer(9_007_199_254_740_993.0));
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
fn number_formatting_rules() {
    assert_eq!(number::to_fixed(1.2345, Some(2.0)).as_deref(), Ok("1.23"));
    assert_eq!(number::to_fixed(2.55, Some(1.0)).as_deref(), Ok("2.5"));
    assert_eq!(number::to_fixed(1e21, Some(2.0)).as_deref(), Ok("1e+21"));
    assert_eq!(
        number::to_fixed(1.234, Some(101.0)).unwrap_err().kind(),
        tsonic_rust_runtime::JsErrorKind::RangeError
    );
    assert_eq!(number::to_string_radix(255_i32, 16.0).as_deref(), Ok("ff"));
    assert_eq!(
        number::to_string_radix(255_i32, 37.0).unwrap_err().kind(),
        tsonic_rust_runtime::JsErrorKind::RangeError
    );
    assert_eq!(
        number::to_exponential(12.5, Some(1.0)).as_deref(),
        Ok("1.3e+1")
    );
    assert_eq!(number::to_exponential(12.5, None).as_deref(), Ok("1.25e+1"));
    assert_eq!(number::to_precision(12.345, Some(2.0)).as_deref(), Ok("12"));
    assert_eq!(
        number::to_precision(2.55, Some(4.0)).as_deref(),
        Ok("2.550")
    );
    assert_eq!(
        number::to_precision(12.345, Some(0.0)).unwrap_err().kind(),
        tsonic_rust_runtime::JsErrorKind::RangeError
    );
}

#[test]
fn same_value_zero_and_strict_equal_for_nans_and_zeroes() {
    assert!(same_value_zero_f64(f64::NAN, f64::NAN));
    assert!(!strict_equal_f64(f64::NAN, f64::NAN));
    assert!(same_value_zero_f64(0.0, -0.0));
    assert!(strict_equal_f64(0.0, -0.0));
}

#[test]
fn parse_float_invalid_leading_sign_combinations() {
    assert!(number::parse_float("-").is_nan());
    assert!(number::parse_float("+").is_nan());
    assert!(number::parse_float("  +Infinity").is_infinite());
    assert!(number::parse_float("0x1.2").is_finite());
    assert_eq!(number::parse_float("0x1.2"), 0.0);
}

#[test]
fn number_to_string_radix_rules() {
    assert_eq!(number::to_string_radix(0_i32, 16.0).as_deref(), Ok("0"));
    assert_eq!(
        number::to_string_radix(-255_i32, 16.0).as_deref(),
        Ok("-ff")
    );
    assert_eq!(
        number::to_string_radix(255_i32, 1.0).unwrap_err().kind(),
        tsonic_rust_runtime::JsErrorKind::RangeError
    );
}

#[test]
fn number_formatting_matches_ecmascript_edge_shapes() {
    assert_eq!(number::to_string(-0.0), "0");
    assert_eq!(number::to_string(1e-7), "1e-7");
    assert_eq!(number::to_string(1e20), "100000000000000000000");
    assert_eq!(number::to_exponential_default(0.0), "0e+0");
    assert_eq!(number::to_exponential_default(-0.0), "0e+0");
    assert_eq!(
        number::to_exponential_digits(1e-7, 2.0).as_deref(),
        Ok("1.00e-7")
    );
    assert_eq!(
        number::to_precision_digits(0.0012, 4.0).as_deref(),
        Ok("0.001200")
    );
    assert_eq!(
        number::to_precision_digits(9.999999999999998, 2.0).as_deref(),
        Ok("10")
    );
    assert_eq!(number::value_of(42_i32), 42);
}
