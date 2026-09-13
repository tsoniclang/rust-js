use tsonic_rust_js::{abi, array::ArrayLength, JsErrorKind};
use tsonic_rust_runtime::BigInt;

#[test]
fn bigint_construction_preserves_all_integer_bits() {
    assert_eq!(
        abi::bigint_from_integer(9_007_199_254_740_993_u64).to_string(),
        "9007199254740993"
    );
    assert_eq!(
        abi::bigint_from_integer(i128::MIN).to_string(),
        i128::MIN.to_string()
    );
    assert_eq!(
        abi::bigint_from_integer(u128::MAX).to_string(),
        u128::MAX.to_string()
    );
    assert_eq!(abi::bigint_from_boolean(true).to_string(), "1");
    assert_eq!(abi::bigint_from_boolean(false).to_string(), "0");
    assert_eq!(abi::bigint_from_number(-42.0).unwrap().to_string(), "-42");
    assert_eq!(abi::bigint_from_number(-0.0).unwrap().to_string(), "0");
    assert_eq!(abi::bigint_from_number(1e100).unwrap().to_string(), "10000000000000000159028911097599180468360808563945281389781327557747838772170381060813469985856815104");
    for number in [1.5, -1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            abi::bigint_from_number(number).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
    }
}

#[test]
fn bigint_string_construction_uses_integer_grammar() {
    for (source, expected) in [
        ("", "0"),
        ("\u{feff}  +42\n", "42"),
        ("-42", "-42"),
        ("0xff", "255"),
        ("0o17", "15"),
        ("0b101", "5"),
        ("9007199254740993", "9007199254740993"),
    ] {
        assert_eq!(
            abi::bigint_from_string(source).unwrap(),
            BigInt::from_decimal_literal(expected)
        );
    }
    for source in [
        "0x", "-0x1", "+0b1", "1.0", "1e3", "1_000", "1n", "++1", "NaN", "\u{85}1",
    ] {
        assert_eq!(
            abi::bigint_from_string(source).unwrap_err().kind(),
            JsErrorKind::SyntaxError
        );
    }
}

#[test]
fn array_length_construction_keeps_holes_and_reference_fill() {
    let values = abi::array_construct_length::<String>(3_i64).unwrap();
    assert_eq!(values.len(), 3);
    assert!(!values.has_index(0));
    let empty = abi::array_construct_length::<String>(-0.0).unwrap();
    assert_eq!(empty.len(), 0);
    let item = abi::array_of([3.0]);
    assert_eq!(item.len(), 1);
    assert_eq!(item.at(0.0), Some(3.0));
    assert_eq!(u32::MAX.array_length().unwrap(), u32::MAX as usize);
    for length in [-1.0, 1.5, f64::NAN, f64::INFINITY, 4_294_967_296.0] {
        assert_eq!(
            abi::array_construct_length::<String>(length)
                .unwrap_err()
                .kind(),
            JsErrorKind::RangeError
        );
    }
    assert!(u128::MAX.array_length().is_err());
    assert!(i128::MIN.array_length().is_err());
}

#[test]
fn empty_objects_retain_identity_and_freeze_through_aliases() {
    use tsonic_rust_js::equality::{JsHash, JsSameValue, JsSameValueZero, JsStrictEqual};
    use tsonic_rust_runtime::ObjectIdentityCarrier;
    let first = abi::EmptyObject::new();
    let alias = first.clone();
    let other = abi::EmptyObject::default();
    assert!(!first.is_frozen());
    assert_eq!(first.freeze(), alias);
    assert!(alias.is_frozen());
    assert!(!other.is_frozen());
    assert!(first.same_value(&alias));
    assert!(first.same_value_zero(&alias));
    assert!(first.strict_equal(&alias));
    assert_eq!(first.js_hash(), alias.js_hash());
    assert_eq!(first.object_identity(), alias.object_identity());
    assert_ne!(first, other);
    let tokens = abi::JsSet::new();
    tokens.add(first.clone());
    assert!(tokens.has(&alias));
    assert!(!tokens.has(&other));
    let backing = abi::array_construct_length::<abi::EmptyObject>(2).unwrap();
    backing.fill_all(first.clone());
    assert_eq!(backing.at(0.0), backing.at(1.0));
    assert_eq!(backing.at(0.0), Some(first));
}
