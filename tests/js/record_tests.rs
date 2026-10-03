use tsonic_rust_js::object::records;
use tsonic_rust_runtime::Record;

#[test]
fn record_projection_preserves_exact_values_and_reference_identity() {
    let target = Record::from_entries([("first".to_owned(), Some(9_007_199_254_740_993_u64))]);
    assert_eq!(records::keys(&target).get(0), Some("first".to_owned()));
    assert_eq!(
        records::values(&target).get(0),
        Some(Some(9_007_199_254_740_993))
    );
    assert_eq!(
        records::entries(&target).get(0),
        Some(("first".to_owned(), Some(9_007_199_254_740_993)))
    );
    let source = Record::from_entries([("second".to_owned(), None)]);
    let result = records::assign(&target, &source);
    assert_eq!(result, target);
    assert!(result.contains_key(&"second".to_owned()));
    assert_eq!(records::assign(&target, &target), target);
}

#[test]
fn closed_record_admission_retains_aliases_live_values_and_absent_membership() {
    use tsonic_rust_js::equality::{JsHash, JsSameValue, JsSameValueZero, JsStrictEqual};
    use tsonic_rust_js::{json, value::closed_value_string, JsValue};
    let original =
        Record::from_entries([(String::from("wide"), JsValue::UnsignedInteger(u64::MAX))]);
    let value = JsValue::from(original.clone());
    let alias = JsValue::from(original.clone());
    let distinct = JsValue::from(Record::from_entries([(
        String::from("wide"),
        JsValue::UnsignedInteger(u64::MAX),
    )]));
    assert!(value.strict_equal(&alias));
    assert!(value.same_value(&alias));
    assert!(value.same_value_zero(&alias));
    assert_eq!(value.js_hash(), alias.js_hash());
    assert!(!value.strict_equal(&distinct));
    assert_eq!(
        value.reference_identity_key(),
        Some(original.storage_identity_key())
    );
    assert_eq!(value.type_of(), "object");
    assert_eq!(closed_value_string(&value).unwrap(), "[object Object]");
    assert_eq!(
        json::stringify(&value).unwrap().unwrap(),
        "{\"wide\":18446744073709551615}"
    );
    original.set(String::from("wide"), JsValue::Null);
    assert_eq!(json::stringify(&value).unwrap().unwrap(), "{\"wide\":null}");
    let restored = alias.as_record().unwrap();
    assert!(restored.contains_key("wide"));
    assert!(!restored.contains_key("missing"));
    assert!(restored.get_or_default("missing").is_nullish());
    restored.remove("wide");
    assert_eq!(json::stringify(&value).unwrap().unwrap(), "{}");
    assert!(JsValue::Null.as_record().is_none());
}

#[test]
fn closed_record_json_retains_limits_cycles_and_live_callback_reads() {
    use tsonic_rust_js::{json, JsArray, JsValue};
    let original = Record::from_entries([(String::from("value"), JsValue::Int32(7))]);
    let value = JsValue::from(original.clone());
    let mut calls = 0;
    let output = json::stringify_with_replacer(&value, |key, selected| {
        calls += 1;
        if key.is_empty() {
            original.set(String::from("value"), JsValue::UnsignedInteger(u64::MAX));
        } else {
            original.set(String::from("added"), JsValue::Bool(true));
        }
        selected
    })
    .unwrap()
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(output, "{\"value\":18446744073709551615}");
    let selected = JsValue::array(JsArray::from_dense(vec![JsValue::String(String::from(
        "value",
    ))]));
    assert_eq!(
        json::stringify_with_property_list(&value, &selected)
            .unwrap()
            .unwrap(),
        output
    );
    for limits in [
        json::JsonLimits {
            max_nodes: 1,
            ..Default::default()
        },
        json::JsonLimits {
            max_members: 0,
            ..Default::default()
        },
        json::JsonLimits {
            max_output_bytes: 2,
            ..Default::default()
        },
        json::JsonLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        assert!(json::stringify_with_limits(&value, limits).is_err());
    }
    original.set(String::from("self"), value.clone());
    assert!(json::stringify(&value).is_err());
    assert!(value.inspect().contains("[Circular]"));
    original.remove("self");
}

#[test]
fn closed_record_intl_options_read_current_exact_native_values() {
    use tsonic_rust_js::{IntlNumberFormat, JsValue};
    let original = Record::from_entries([
        (String::from("maximumFractionDigits"), JsValue::Uint8(0)),
        (String::from("useGrouping"), JsValue::Bool(false)),
    ]);
    let options = JsValue::from(original.clone());
    let first = IntlNumberFormat::with_locale_options("en", &options).unwrap();
    assert_eq!(first.format(u64::MAX), "18446744073709551615");
    assert_eq!(first.resolved_options().maximum_fraction_digits(), Some(0));
    original.set(String::from("useGrouping"), JsValue::Bool(true));
    let grouped = IntlNumberFormat::with_locale_options("en", &options).unwrap();
    assert_eq!(grouped.format(u64::MAX), "18,446,744,073,709,551,615");
    original.set(
        String::from("maximumFractionDigits"),
        JsValue::UnsignedInteger(u64::MAX),
    );
    assert!(IntlNumberFormat::with_locale_options("en", &options).is_err());
    original.set(String::from("maximumFractionDigits"), JsValue::Null);
    let absent = IntlNumberFormat::with_locale_options("en", &options).unwrap();
    assert_eq!(absent.resolved_options().maximum_fraction_digits(), Some(3));
    assert!(IntlNumberFormat::with_locale_options("en", &JsValue::Bool(false)).is_err());
}
