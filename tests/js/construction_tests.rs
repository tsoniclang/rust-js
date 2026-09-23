use tsonic_rust_js::{abi, array::ArrayLength, JsErrorKind};
use tsonic_rust_runtime::BigInt;

#[test]
fn compiler_provider_error_constructors_retain_kind_and_message() {
    for (error, expected) in [
        (abi::range_error("range"), JsErrorKind::RangeError),
        (abi::type_error("type"), JsErrorKind::TypeError),
        (abi::uri_error("uri"), JsErrorKind::URIError),
    ] {
        assert_eq!(error.kind(), expected);
        assert!(!error.message().is_empty());
    }
    assert_eq!(abi::range_error("").message(), "");
}

#[test]
fn bigint_width_selection_preserves_exact_signed_and_unsigned_bits() {
    for (bits, input, signed, unsigned) in [
        (0.0, "-17", "0", "0"),
        (1.0, "3", "-1", "1"),
        (7.0, "64", "-64", "64"),
        (7.0, "-129", "-1", "127"),
        (8.0, "255", "-1", "255"),
        (8.0, "-129", "127", "127"),
        (9.0, "256", "-256", "256"),
        (9.0, "-1", "-1", "511"),
        (64.0, "18446744073709551615", "-1", "18446744073709551615"),
        (
            64.0,
            "-9223372036854775809",
            "9223372036854775807",
            "9223372036854775807",
        ),
        (
            64.0,
            "9007199254740993",
            "9007199254740993",
            "9007199254740993",
        ),
    ] {
        let value = BigInt::from_decimal_literal(input);
        assert_eq!(
            abi::bigint_as_int_n(bits, &value).unwrap().to_string(),
            signed
        );
        assert_eq!(
            abi::bigint_as_uint_n(bits, &value).unwrap().to_string(),
            unsigned
        );
    }
    let value = BigInt::from_decimal_literal("9");
    for width in [0.0, -0.0] {
        assert_eq!(
            abi::bigint_as_int_n(width, &value).unwrap().to_string(),
            "0"
        );
        assert_eq!(
            abi::bigint_as_uint_n(width, &value).unwrap().to_string(),
            "0"
        );
    }
    for width in [
        -1.0,
        -0.5,
        1.9,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        18_446_744_073_709_551_616.0,
    ] {
        assert_eq!(
            abi::bigint_as_int_n(width, &value).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
        assert_eq!(
            abi::bigint_as_uint_n(width, &value).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
    }
    assert_eq!(
        abi::bigint_as_int_n(9_007_199_254_740_992.0, &value).unwrap(),
        value
    );
    assert_eq!(
        abi::bigint_as_uint_n(9_007_199_254_740_992.0, &value).unwrap(),
        value
    );
}

#[test]
fn bigint_radix_formatting_retains_all_integer_bits() {
    let value = abi::bigint_from_string("-9007199254740993").unwrap();
    assert_eq!(
        abi::bigint_to_string_radix(&value, 16.0).unwrap(),
        "-20000000000001"
    );
    assert_eq!(
        abi::bigint_to_string_radix(&value, 2.0).unwrap(),
        format!("-1{}1", "0".repeat(52))
    );
    let value = abi::bigint_from_string("35").unwrap();
    assert_eq!(abi::bigint_to_string_radix(&value, 36.0).unwrap(), "z");
    for radix in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -2.0, 1.0, 37.0] {
        assert_eq!(
            abi::bigint_to_string_radix(&value, radix)
                .unwrap_err()
                .kind(),
            JsErrorKind::RangeError
        );
    }
}

#[test]
fn native_bigint_operands_truncate_without_losing_high_bits() {
    macro_rules! pair {
        ($unsigned:expr, $signed:expr, $bits:expr) => {
            assert_eq!(abi::bigint_as_int_n($bits, &$unsigned).unwrap().to_string(), "-1");
            assert_eq!(abi::bigint_as_uint_n($bits, &$signed).unwrap(), abi::bigint_from_integer($unsigned));
        };
    }
    pair!(u8::MAX, -1_i8, 8.0);
    pair!(u16::MAX, -1_i16, 16.0);
    pair!(u32::MAX, -1_i32, 32.0);
    pair!(u64::MAX, -1_i64, 64.0);
    pair!(u128::MAX, -1_i128, 128.0);
    pair!(usize::MAX, -1_isize, usize::BITS as f64);
    assert_eq!(abi::bigint_as_uint_n(129.0, &-1_i64).unwrap().to_string(), "680564733841876926926749214863536422911");
    assert_eq!(abi::bigint_as_int_n(256.0, &-1_i64).unwrap().to_string(), "-1");
    assert_eq!(abi::bigint_as_uint_n(9_007_199_254_740_992.0, &9_u64).unwrap().to_string(), "9");
    assert_eq!(abi::bigint_as_int_n(f64::NAN, &9_u64).unwrap_err().kind(), JsErrorKind::RangeError);
    assert_eq!(abi::bigint_as_int_n(-1.0, &9_u64).unwrap_err().kind(), JsErrorKind::RangeError);
}

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
    assert_eq!(
        abi::bigint_from_number(1e100).unwrap().to_string(),
        "10000000000000000159028911097599180468360808563945281389781327557747838772170381060813469985856815104"
    );
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
fn array_length_construction_initializes_native_values() {
    let values = abi::array_construct_length::<String>(3_i64).unwrap();
    assert_eq!(values.len(), 3);
    assert!(values.has_index(0));
    assert_eq!(values.get(0), Some(String::new()));
    let empty = abi::array_construct_length::<String>(-0.0).unwrap();
    assert_eq!(empty.len(), 0);
    let item = abi::array_of([3.0]);
    assert_eq!(item.len(), 1);
    assert_eq!(item.at(0.0), Some(3.0));
    assert_eq!(u32::MAX.array_length().unwrap(), u32::MAX as usize);
    for length in [-1.0, 1.5, f64::NAN, f64::INFINITY, (usize::MAX as u128 + 1) as f64] {
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
    let first = tsonic_rust_runtime::EmptyObject::new();
    let alias = first.clone();
    let other = tsonic_rust_runtime::EmptyObject::default();
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
    let backing = abi::array_construct_length::<tsonic_rust_runtime::EmptyObject>(2).unwrap();
    backing.fill_all(first.clone());
    assert_eq!(backing.at(0.0), backing.at(1.0));
    assert_eq!(backing.at(0.0), Some(first));
}

#[test]
fn empty_objects_keep_identity_when_boxed_as_closed_values() {
    use tsonic_rust_js::value::{JsClosedValueCarrier, JsValue};
    let first = tsonic_rust_runtime::EmptyObject::new();
    let alias = first.clone();
    let other = tsonic_rust_runtime::EmptyObject::new();
    let boxed = abi::js_value_from_closed(&first);
    assert_eq!(boxed, abi::js_value_from_closed(&alias));
    assert_ne!(boxed, abi::js_value_from_closed(&other));
    first.freeze();
    assert!(alias.is_frozen());
    assert_eq!(boxed, abi::js_value_from_closed(&alias));
    assert_eq!(first.inspect_value(), "[object Object]");
    assert_eq!(
        tsonic_rust_js::json::stringify(&first.project_json().unwrap()).unwrap(),
        Some("{}".to_owned())
    );
    assert_ne!(boxed, JsValue::Undefined);
}
