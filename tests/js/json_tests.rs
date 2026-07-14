use tsonic_rust_js::{json, JsErrorKind, JsObject, JsValue};

fn stringify_text(value: &JsValue) -> String {
    json::stringify(value).unwrap().unwrap()
}

#[test]
fn json_parse_and_stringify_closed_values() {
    let value = json::parse(r#"{"a":1,"b":[true,null]}"#).unwrap();
    let JsValue::Object(object) = &value else {
        panic!("expected object");
    };
    assert_eq!(object.borrow().get("a"), JsValue::Number(1.0));

    let text = stringify_text(&value);
    assert_eq!(text, r#"{"a":1,"b":[true,null]}"#);
    assert_eq!(
        json::stringify_pretty(&json::parse(r#"{"pretty":false}"#).unwrap())
            .unwrap()
            .unwrap(),
        r#"{"pretty":false}"#
    );
}

#[test]
fn json_omits_undefined_object_fields_and_nulls_array_slots() {
    assert!(JsValue::Undefined.is_nullish());
    assert!(JsValue::Null.is_nullish());
    assert!(!JsValue::Bool(false).is_nullish());
    let object =
        JsObject::from_pairs([("keep", JsValue::Number(1.0)), ("skip", JsValue::Undefined)]);
    assert_eq!(stringify_text(&JsValue::object(object)), r#"{"keep":1}"#);
    assert_eq!(
        stringify_text(&JsValue::from(vec![JsValue::Undefined])),
        "[null]"
    );
}

#[test]
fn json_round_trips_non_ascii_strings() {
    let text = "héllo — ünïcode ✓";
    let parsed = json::parse("\"héllo — ünïcode ✓\"").unwrap();
    assert_eq!(parsed, JsValue::String(text.to_string()));

    let source = r#"{"msg":"héllo — ünïcode ✓"}"#;
    let value = json::parse(source).unwrap();
    assert_eq!(
        value.as_object().expect("object").borrow().get("msg"),
        JsValue::String(text.to_string())
    );
    let round_tripped = stringify_text(&value);
    assert_eq!(round_tripped, source);
    let reparsed = json::parse(&round_tripped).unwrap();
    assert_eq!(
        reparsed.as_object().expect("object").borrow().get("msg"),
        JsValue::String(text.to_string())
    );
}

#[test]
fn json_rejects_invalid_input() {
    assert!(json::parse("{").is_err());
}

#[test]
fn json_stringify_with_indent_matches_node_output() {
    // Expected strings generated with Node:
    // JSON.stringify(JSON.parse(source), null, space).
    let value =
        json::parse(r#"{"a":1,"b":[true,null,[]],"c":{"d":"x","e":{}},"f":"line\n\ttab "}"#)
            .unwrap();

    assert_eq!(
        json::stringify_with_indent(&value, "  ").unwrap().unwrap(),
        "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    null,\n    []\n  ],\n  \"c\": {\n    \"d\": \"x\",\n    \"e\": {}\n  },\n  \"f\": \"line\\n\\ttab \"\n}"
    );
    assert_eq!(
        json::stringify_with_indent(&value, "\t").unwrap().unwrap(),
        "{\n\t\"a\": 1,\n\t\"b\": [\n\t\ttrue,\n\t\tnull,\n\t\t[]\n\t],\n\t\"c\": {\n\t\t\"d\": \"x\",\n\t\t\"e\": {}\n\t},\n\t\"f\": \"line\\n\\ttab \"\n}"
    );
    // Empty indent is exactly the compact form.
    assert_eq!(
        json::stringify_with_indent(&value, "").unwrap().unwrap(),
        r#"{"a":1,"b":[true,null,[]],"c":{"d":"x","e":{}},"f":"line\n\ttab "}"#
    );
    assert_eq!(
        json::stringify_with_indent(&value, "").unwrap().unwrap(),
        stringify_text(&value)
    );
}

#[test]
fn json_stringify_with_indent_nested_arrays_and_leaves() {
    let nested = json::parse("[1,[2,[3]]]").unwrap();
    assert_eq!(
        json::stringify_with_indent(&nested, "    ")
            .unwrap()
            .unwrap(),
        "[\n    1,\n    [\n        2,\n        [\n            3\n        ]\n    ]\n]"
    );

    // Empty containers never break onto new lines.
    assert_eq!(
        json::stringify_with_indent(&json::parse("{}").unwrap(), "  ")
            .unwrap()
            .unwrap(),
        "{}"
    );
    assert_eq!(
        json::stringify_with_indent(&json::parse("[]").unwrap(), "  ")
            .unwrap()
            .unwrap(),
        "[]"
    );

    // Scalars are unaffected by the indent.
    assert_eq!(
        json::stringify_with_indent(&JsValue::String("plain".to_string()), "  ")
            .unwrap()
            .unwrap(),
        "\"plain\""
    );
    assert_eq!(
        json::stringify_with_indent(&JsValue::Number(7.0), "  ")
            .unwrap()
            .unwrap(),
        "7"
    );
    assert_eq!(
        json::stringify_with_indent(&JsValue::Undefined, "  ").unwrap(),
        None
    );
}

#[test]
fn json_stringify_with_indent_keeps_undefined_member_rules() {
    let object =
        JsObject::from_pairs([("keep", JsValue::Number(2.0)), ("skip", JsValue::Undefined)]);
    assert_eq!(
        json::stringify_with_indent(&JsValue::object(object), "  ")
            .unwrap()
            .unwrap(),
        "{\n  \"keep\": 2\n}"
    );
    assert_eq!(
        json::stringify_with_indent(&JsValue::from(vec![JsValue::Undefined]), "  ")
            .unwrap()
            .unwrap(),
        "[\n  null\n]"
    );
}

#[test]
fn json_quotes_control_characters_like_node() {
    let value = JsValue::String("\u{1}\u{1f}\u{8}\u{c}\"\\".to_string());
    assert_eq!(stringify_text(&value), "\"\\u0001\\u001f\\b\\f\\\"\\\\\"");
}

#[test]
fn json_rejects_cycles_and_borrow_conflicts_but_allows_shared_aliases() {
    let cycle = JsValue::object(JsObject::new());
    cycle
        .as_object()
        .unwrap()
        .borrow_mut()
        .set("self", cycle.clone());
    let error = json::stringify(&cycle).unwrap_err();
    assert_eq!(error.kind(), JsErrorKind::TypeError);
    cycle.as_object().unwrap().borrow_mut().delete("self");

    let array_cycle = JsValue::from(Vec::<JsValue>::new());
    array_cycle
        .as_array()
        .unwrap()
        .borrow_mut()
        .set(0, array_cycle.clone());
    assert_eq!(
        json::stringify(&array_cycle).unwrap_err().kind(),
        JsErrorKind::TypeError
    );
    array_cycle.as_array().unwrap().borrow_mut().delete_at(0);

    let child = JsValue::object(JsObject::from_pairs([("x", JsValue::Number(1.0))]));
    let shared = JsValue::object(JsObject::from_pairs([("a", child.clone()), ("b", child)]));
    assert_eq!(stringify_text(&shared), r#"{"a":{"x":1},"b":{"x":1}}"#);

    let borrowed = JsValue::object(JsObject::new());
    let handle = borrowed.as_object().unwrap().clone();
    let _guard = handle.borrow_mut();
    assert_eq!(
        json::stringify(&borrowed).unwrap_err().kind(),
        JsErrorKind::TypeError
    );
}

#[test]
fn json_enforces_resource_limits() {
    let limits = json::JsonLimits {
        max_input_bytes: 5,
        max_output_bytes: 8,
        max_depth: 1,
        max_nodes: 4,
        max_members: 2,
    };
    assert!(json::parse_with_limits("[[]]", limits).is_ok());
    assert_eq!(
        json::parse_with_limits("[[[]]]", limits)
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        json::parse_with_limits("[1,2,3]", limits)
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        json::parse_with_limits("123456", limits)
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        json::stringify_with_limits(&JsValue::String("\u{1}".to_string()), limits)
            .unwrap()
            .unwrap(),
        "\"\\u0001\""
    );
    assert_eq!(
        json::stringify_with_limits(
            &JsValue::String("\u{1}".to_string()),
            json::JsonLimits {
                max_output_bytes: 7,
                ..limits
            },
        )
        .unwrap_err()
        .kind(),
        JsErrorKind::RangeError
    );
}

#[test]
fn json_number_grammar_and_output_match_ecmascript() {
    for invalid in ["01", "-01", "00", "1.", "1.e2", "-.1", "1e", "1e+", "+1"] {
        assert_eq!(
            json::parse(invalid).unwrap_err().kind(),
            JsErrorKind::SyntaxError
        );
    }
    for valid in ["0", "-0", "0.0", "1E+2", "1e-2", "1e400"] {
        assert!(json::parse(valid).is_ok(), "{valid}");
    }
    for (value, expected) in [
        (-0.0, "0"),
        (1e21, "1e+21"),
        (1e-7, "1e-7"),
        (1_000_000_000_000_000_128.0, "1000000000000000100"),
        (f64::from_bits(1), "5e-324"),
        (f64::NAN, "null"),
        (f64::INFINITY, "null"),
        (f64::NEG_INFINITY, "null"),
    ] {
        assert_eq!(
            stringify_text(&JsValue::Number(value)),
            expected,
            "{value:?}"
        );
    }
}

#[test]
fn json_utf16_escape_policy_is_explicit() {
    assert_eq!(
        json::parse(r#""\uD83D\uDE00""#).unwrap(),
        JsValue::String("😀".to_string())
    );
    assert_eq!(
        json::parse(r#""\u12G4""#).unwrap_err().kind(),
        JsErrorKind::SyntaxError
    );
    for value in [r#""\uD800""#, r#""\uDC00""#] {
        assert_eq!(
            json::parse(value).unwrap_err().kind(),
            JsErrorKind::Unsupported
        );
    }
}
