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

#[test]
fn closed_record_json_releases_the_backing_before_nested_native_projections() {
    use tsonic_rust_js::{json, value::js_value_from_json_projection, JsArray, JsValue};
    let original = Record::default();
    let projected = js_value_from_json_projection(original.clone(), |record, key| {
        assert_eq!(key, "0");
        record.set(String::from("value"), JsValue::Null);
        record.set(String::from("added"), JsValue::Bool(false));
        Ok(JsValue::Bool(true))
    });
    original.set(
        String::from("value"),
        JsValue::array(JsArray::from_dense(vec![projected])),
    );
    let value = JsValue::from(original.clone());
    assert_eq!(
        json::stringify(&value).unwrap().as_deref(),
        Some("{\"value\":[true]}")
    );
    assert!(original.get("value").is_nullish());
    assert_eq!(original.get("added"), JsValue::Bool(false));
}

#[test]
fn closed_record_json_snapshots_only_unprocessed_members_before_native_projection() {
    use std::cell::Cell;
    use std::rc::Rc;
    use tsonic_rust_js::{json, value::js_value_from_json_projection, JsValue};
    let record =
        Record::from_entries((0..8).map(|index| (format!("field_{index}"), JsValue::Bool(true))));
    let keys = record.keys();
    let selected_key = keys[3].clone();
    let removed_key = keys.last().unwrap().clone();
    let calls = Rc::new(Cell::new(0));
    let projected = js_value_from_json_projection(
        (record.clone(), keys.clone(), calls.clone()),
        |(record, keys, calls), key| {
            calls.set(calls.get() + 1);
            assert_eq!(key, keys[3]);
            for key in keys {
                record.set(key.clone(), JsValue::Bool(false));
            }
            record.set(keys[4].clone(), JsValue::Null);
            record.remove(keys.last().unwrap());
            record.set(String::from("added"), JsValue::Bool(true));
            Ok(JsValue::Int32(7))
        },
    );
    record.set(selected_key.clone(), projected);
    let output = json::stringify(&JsValue::from(record.clone()))
        .unwrap()
        .unwrap();
    let parsed = json::parse(&output).unwrap();
    let fields = parsed.as_object().unwrap().borrow();
    for key in &keys[..3] {
        assert_eq!(fields.get(key), JsValue::Bool(true));
        assert_eq!(record.get(key), JsValue::Bool(false));
    }
    assert_eq!(fields.get(&selected_key), JsValue::Int32(7));
    assert!(fields.has_own_property(&keys[4]));
    assert!(fields.get(&keys[4]).is_nullish());
    for key in &keys[5..7] {
        assert_eq!(fields.get(key), JsValue::Bool(false));
    }
    assert!(!fields.has_own_property(&removed_key));
    assert!(!fields.has_own_property("added"));
    assert!(!record.contains_key(&removed_key));
    assert!(record.contains_key("added"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn closed_record_json_replacer_can_restore_an_original_key_deleted_by_an_earlier_callback() {
    use tsonic_rust_js::{json, JsValue};
    let record =
        Record::from_entries((0..3).map(|index| (format!("field_{index}"), JsValue::Bool(true))));
    let keys = record.keys();
    let mut restored = false;
    let output = json::stringify_with_replacer(&JsValue::from(record.clone()), |key, value| {
        if key == keys[0] {
            assert!(record.remove(&keys[1]));
            record.set(keys[2].clone(), JsValue::Null);
            record.set(String::from("added"), JsValue::Bool(false));
        } else if key == keys[1] {
            assert!(value.is_nullish());
            restored = true;
            return JsValue::Int32(19);
        }
        value
    })
    .unwrap()
    .unwrap();
    let parsed = json::parse(&output).unwrap();
    let fields = parsed.as_object().unwrap().borrow();
    assert!(restored);
    assert_eq!(fields.get(&keys[0]), JsValue::Bool(true));
    assert_eq!(fields.get(&keys[1]), JsValue::Int32(19));
    assert!(fields.has_own_property(&keys[2]));
    assert!(fields.get(&keys[2]).is_nullish());
    assert!(!fields.has_own_property("added"));
    assert!(!record.contains_key(&keys[1]));
}

#[test]
fn closed_record_intl_reads_share_the_existing_native_read_borrow() {
    use tsonic_rust_js::{IntlNumberFormat, JsValue};
    let record = Record::from_entries([(String::from("useGrouping"), JsValue::Bool(false))]);
    let options = JsValue::from(record.clone());
    let rendered = record.with_entries(|entries| {
        assert_eq!(entries.get("useGrouping"), Some(&JsValue::Bool(false)));
        IntlNumberFormat::with_locale_options("en", &options)
            .unwrap()
            .format(u64::MAX)
    });
    assert_eq!(rendered, "18446744073709551615");
    let object = JsValue::object(tsonic_rust_js::JsObject::from_pairs([(
        "useGrouping",
        false,
    )]));
    let borrowed = object.as_object().unwrap().borrow_mut();
    let error = IntlNumberFormat::with_locale_options("en", &object).unwrap_err();
    assert_eq!(error.kind(), tsonic_rust_js::JsErrorKind::TypeError);
    drop(borrowed);
}

#[test]
fn closed_record_intl_retains_native_exclusive_reentrancy_failure() {
    use tsonic_rust_js::{value::js_value_from_json_projection, IntlNumberFormat, JsValue};
    struct ReentrantRead {
        record: Record<String, JsValue>,
        read: fn(&Record<String, JsValue>),
    }
    impl Drop for ReentrantRead {
        fn drop(&mut self) {
            (self.read)(&self.record);
        }
    }
    let readers: [fn(&Record<String, JsValue>); 2] = [
        |record| {
            record.get_or_default("useGrouping");
        },
        |record| {
            let _result =
                IntlNumberFormat::with_locale_options("en", &JsValue::from(record.clone()));
        },
    ];
    for read in readers {
        let record = Record::from_entries([(String::from("useGrouping"), JsValue::Bool(false))]);
        let projected = js_value_from_json_projection(
            ReentrantRead {
                record: record.clone(),
                read,
            },
            |_source, _key| Ok(JsValue::Null),
        );
        record.set(String::from("probe"), projected);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            record.set(String::from("probe"), JsValue::Null);
        }));
        assert!(result.is_err());
        assert!(record.get("probe").is_nullish());
        assert_eq!(record.get("useGrouping"), JsValue::Bool(false));
    }
}
