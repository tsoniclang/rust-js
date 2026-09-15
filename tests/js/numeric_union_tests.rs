use tsonic_rust_js::abi::SourceNumeric;
use tsonic_rust_js::abi::{bigint_to_number, JsNumeric};
use tsonic_rust_runtime::BigInt;

#[test]
fn constrained_numeric_constructors_preserve_exact_conversion_rules() {
    fn integer<Value: SourceNumeric>(value: &Value) -> tsonic_rust_js::errors::JsResult<BigInt> {
        SourceNumeric::to_bigint(value)
    }
    let exact = BigInt::from_decimal_literal("9007199254740993");
    assert_eq!(integer(&exact).unwrap(), exact);
    assert_eq!(
        integer(&u64::MAX).unwrap(),
        BigInt::from_decimal_literal("18446744073709551615")
    );
    assert_eq!(integer(&7_f64).unwrap(), BigInt::from_decimal_literal("7"));
    assert_eq!(integer(&-0_f64).unwrap(), BigInt::from_decimal_literal("0"));
    assert_eq!(SourceNumeric::to_number(&exact), 9007199254740992_f64);
    for invalid in [0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(integer(&invalid).is_err());
    }
}

#[test]
fn generic_numeric_constraints_preserve_exact_domains() {
    let large = BigInt::from_decimal_literal("9007199254740993");
    assert!(SourceNumeric::greater_than(&large, &9007199254740992_f64));
    assert!(SourceNumeric::less_than(&9007199254740992_f64, &large));
    assert!(SourceNumeric::loose_equal(
        &2_f64,
        &BigInt::from_decimal_literal("2")
    ));
    assert!(!SourceNumeric::strict_equal(
        &2_f64,
        &BigInt::from_decimal_literal("2")
    ));
    assert!(SourceNumeric::strict_equal(&2_u32, &2_f64));
    assert!(SourceNumeric::strict_equal(
        &u64::MAX,
        &BigInt::from_decimal_literal("18446744073709551615")
    ));
    assert!(SourceNumeric::strict_equal(
        &u128::MAX,
        &BigInt::from_decimal_literal("340282366920938463463374607431768211455")
    ));
    assert!(SourceNumeric::strict_equal(
        &i128::MIN,
        &BigInt::from_decimal_literal("-170141183460469231731687303715884105728")
    ));
    assert!(!SourceNumeric::greater_than_or_equal(&f64::NAN, &large));
    assert!(!SourceNumeric::less_than_or_equal(&large, &f64::NAN));
    assert!(SourceNumeric::less_than(&large, &f64::INFINITY));
    assert!(SourceNumeric::greater_than(&large, &f64::NEG_INFINITY));
    assert!(SourceNumeric::loose_not_equal(
        &large,
        &9007199254740992_f64
    ));
    assert!(SourceNumeric::strict_not_equal(
        &large,
        &9007199254740992_f64
    ));
    assert!(SourceNumeric::less_than(
        &BigInt::from_decimal_literal("2"),
        &2.5_f64
    ));
}

#[test]
fn numeric_union_string_conversion_retains_number_and_bigint_semantics() {
    for (value, expected) in [
        (0.0, "0"),
        (-0.0, "0"),
        (1.5, "1.5"),
        (1e21, "1e+21"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
    ] {
        assert_eq!(
            tsonic_rust_runtime::source_string(&JsNumeric::from_number(value)),
            expected
        );
    }
    for expected in ["9007199254740993", "-18446744073709551617"] {
        let value = JsNumeric::from_bigint(&BigInt::from_decimal_literal(expected));
        assert_eq!(tsonic_rust_runtime::source_string(&value), expected);
    }
}

#[test]
fn numeric_union_conversion_and_refinement_preserve_precision() {
    let big = BigInt::from_decimal_literal("9007199254740993");
    let value = JsNumeric::from_bigint(&big);
    assert_eq!(value.type_of(), "bigint");
    assert_eq!(value.as_bigint(), big);
    assert_eq!(value.to_bigint().unwrap(), big);
    assert_eq!(value.to_number(), 9007199254740992.0);
    assert_eq!(bigint_to_number(&big), value.to_number());
    let number = JsNumeric::from_int32(2);
    assert_eq!(number.type_of(), "number");
    assert_eq!(number.as_number(), 2.0);
    assert_eq!(
        number.to_bigint().unwrap(),
        BigInt::from_decimal_literal("2")
    );
    assert!(JsNumeric::from_number(2.5).to_bigint().is_err());
}

#[test]
fn numeric_union_comparison_does_not_round_bigints() {
    let exact = JsNumeric::from_bigint(&BigInt::from_decimal_literal("9007199254740993"));
    let rounded = JsNumeric::from_number(9007199254740992.0);
    assert!(exact.greater_than(&rounded));
    assert!(rounded.less_than(&exact));
    assert!(!exact.less_than_or_equal(&rounded));
    assert!(!rounded.greater_than_or_equal(&exact));
    let big_two = JsNumeric::from_bigint(&BigInt::from_decimal_literal("2"));
    let two = JsNumeric::from_number(2.0);
    assert!(big_two.loose_equal(&two));
    assert!(!big_two.strict_equal(&two));
    assert!(big_two.strict_not_equal(&two));
    assert!(!big_two.loose_not_equal(&two));
    assert!(big_two.less_than(&JsNumeric::from_number(2.5)));
    let negative = JsNumeric::from_bigint(&BigInt::from_decimal_literal("-2"));
    assert!(negative.greater_than(&JsNumeric::from_number(-2.5)));
    assert!(exact.less_than(&JsNumeric::from_number(f64::INFINITY)));
    assert!(exact.greater_than(&JsNumeric::from_number(f64::NEG_INFINITY)));
    let nan = JsNumeric::from_number(f64::NAN);
    assert!(!nan.loose_equal(&nan) && !nan.strict_equal(&nan));
    assert!(!nan.less_than(&two) && !nan.greater_than_or_equal(&two));
}
