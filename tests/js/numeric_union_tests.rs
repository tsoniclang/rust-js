use tsonic_rust_js::abi::{bigint_to_number, JsNumeric};
use tsonic_rust_runtime::BigInt;

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
