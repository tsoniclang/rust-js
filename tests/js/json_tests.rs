use tsonic_rust_js::{json, JsObject, JsValue};

#[test]
fn json_parse_and_stringify_closed_values() {
    let value = json::parse(r#"{"a":1,"b":[true,null]}"#).unwrap();
    let JsValue::Object(object) = &value else {
        panic!("expected object");
    };
    assert_eq!(object.borrow().get("a"), JsValue::Number(1.0));

    let text = json::stringify(&value).unwrap();
    assert_eq!(text, r#"{"a":1,"b":[true,null]}"#);
    assert_eq!(
        json::stringify_pretty(&json::parse(r#"{"pretty":false}"#).unwrap()).unwrap(),
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
    assert_eq!(
        json::stringify(&JsValue::object(object)).unwrap(),
        r#"{"keep":1}"#
    );
    assert_eq!(
        json::stringify(&JsValue::from(vec![JsValue::Undefined])).unwrap(),
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
    let round_tripped = json::stringify(&value).unwrap();
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
        json::stringify_with_indent(&value, "  ").unwrap(),
        "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    null,\n    []\n  ],\n  \"c\": {\n    \"d\": \"x\",\n    \"e\": {}\n  },\n  \"f\": \"line\\n\\ttab \"\n}"
    );
    assert_eq!(
        json::stringify_with_indent(&value, "\t").unwrap(),
        "{\n\t\"a\": 1,\n\t\"b\": [\n\t\ttrue,\n\t\tnull,\n\t\t[]\n\t],\n\t\"c\": {\n\t\t\"d\": \"x\",\n\t\t\"e\": {}\n\t},\n\t\"f\": \"line\\n\\ttab \"\n}"
    );
    // Empty indent is exactly the compact form.
    assert_eq!(
        json::stringify_with_indent(&value, "").unwrap(),
        r#"{"a":1,"b":[true,null,[]],"c":{"d":"x","e":{}},"f":"line\n\ttab "}"#
    );
    assert_eq!(
        json::stringify_with_indent(&value, "").unwrap(),
        json::stringify(&value).unwrap()
    );
}

#[test]
fn json_stringify_with_indent_nested_arrays_and_leaves() {
    let nested = json::parse("[1,[2,[3]]]").unwrap();
    assert_eq!(
        json::stringify_with_indent(&nested, "    ").unwrap(),
        "[\n    1,\n    [\n        2,\n        [\n            3\n        ]\n    ]\n]"
    );

    // Empty containers never break onto new lines.
    assert_eq!(
        json::stringify_with_indent(&json::parse("{}").unwrap(), "  ").unwrap(),
        "{}"
    );
    assert_eq!(
        json::stringify_with_indent(&json::parse("[]").unwrap(), "  ").unwrap(),
        "[]"
    );

    // Scalars are unaffected by the indent.
    assert_eq!(
        json::stringify_with_indent(&JsValue::String("plain".to_string()), "  ").unwrap(),
        "\"plain\""
    );
    assert_eq!(
        json::stringify_with_indent(&JsValue::Number(7.0), "  ").unwrap(),
        "7"
    );
    // JSON.stringify(undefined, null, 2) is undefined: mapped to "".
    assert_eq!(
        json::stringify_with_indent(&JsValue::Undefined, "  ").unwrap(),
        ""
    );
}

#[test]
fn json_stringify_with_indent_keeps_undefined_member_rules() {
    let object =
        JsObject::from_pairs([("keep", JsValue::Number(2.0)), ("skip", JsValue::Undefined)]);
    assert_eq!(
        json::stringify_with_indent(&JsValue::object(object), "  ").unwrap(),
        "{\n  \"keep\": 2\n}"
    );
    assert_eq!(
        json::stringify_with_indent(&JsValue::from(vec![JsValue::Undefined]), "  ").unwrap(),
        "[\n  null\n]"
    );
}

#[test]
fn json_quotes_control_characters_like_node() {
    let value = JsValue::String("\u{1}\u{1f}\u{8}\u{c}\"\\".to_string());
    assert_eq!(
        json::stringify(&value).unwrap(),
        "\"\\u0001\\u001f\\b\\f\\\"\\\\\""
    );
}
