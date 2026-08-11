use tsonic_rust_js as js;

#[test]
fn js_backend_legal_abi_paths_are_emit_ready() {
    let dense = js::abi::JsArray::from_dense(vec![1_i32, 2_i32]);
    assert_eq!(dense.push(3), 3);
    assert_eq!(dense.at(-1.0), Some(3));
    assert_eq!(
        dense.map(|x| x * 2).values(),
        vec![Some(2), Some(4), Some(6)]
    );
    assert!(dense.includes(&2, 0.0));
    assert_eq!(dense.index_of(&3, 0.0), 2);
    assert_eq!(dense.join(","), "1,2,3");
    assert_eq!(dense.slice(1.0, None).values(), vec![Some(2), Some(3)]);
    assert_eq!(dense.slice_to(0.0, 2.0).values(), vec![Some(1), Some(2)]);
    assert!(js::abi::number_is_finite(1.0));
    assert!(js::abi::number_is_integer(1.0));
    assert!(!js::abi::number_is_nan(1.0));
    assert!(js::abi::number_is_safe_integer(1.0));

    let mut out = Vec::new();
    js::abi::console_log_to(&mut out, &[js::abi::JsValue::String("ok".to_string())]).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "ok\n");
    let text = String::from("kept");
    let converted = js::abi::js_value_from_string(&text);
    let cloned = js::abi::clone_js_value(&converted);
    assert_eq!(text, "kept");
    assert_eq!(converted, cloned);

    let parsed = js::abi::json_parse(r#"{"ok":true}"#).unwrap();
    let text = js::abi::json_stringify(&parsed).unwrap().unwrap();
    assert_eq!(text, r#"{"ok":true}"#);

    let map = js::abi::JsMap::<f64, &str>::new();
    map.set(f64::NAN, "nan");
    assert_eq!(map.get(&f64::NAN), Some("nan"));

    let set = js::abi::JsSet::<f64>::new();
    set.add(f64::NAN);
    assert!(set.has(&f64::NAN));

    assert_eq!(
        js::abi::JsDate::from_millis(0.0).to_iso_string().unwrap(),
        "1970-01-01T00:00:00.000Z"
    );
    let re = js::abi::JsRegExp::new("a(b+)c", "g").unwrap();
    assert!(re.test("xabbc").unwrap());
    assert_eq!(re.find_first("xabbc").unwrap(), Some((1, 5)));
    assert_eq!(re.replace("abc abbc", "[$1]").unwrap(), "[b] [bb]");
    assert_eq!(re.search("xabc").unwrap(), 1);
    assert_eq!(
        js::abi::JsRegExp::new(",", "")
            .unwrap()
            .split("a,b")
            .unwrap()
            .values(),
        vec![Some("a".to_string()), Some("b".to_string())]
    );

    assert_eq!(dense.find_index(|x| x == 2), 1);
    assert_eq!(dense.find(|x| x == 2), Some(2));
    assert_eq!(dense.find_last(|x| x < 3), Some(2));
    assert_eq!(dense.find_last_index(|x| x < 3), 1);

    assert_eq!(
        js::abi::json_stringify_with_indent(&parsed, "  ")
            .unwrap()
            .unwrap(),
        "{\n  \"ok\": true\n}"
    );

    let algebra = js::abi::JsSet::from_values([1, 2]);
    assert_eq!(algebra.union(&js::abi::JsSet::from_values([3])).len(), 3);
    assert!(algebra.is_superset_of(&js::abi::JsSet::from_values([1])));

    assert_eq!(js::abi::JsDate::parse("1970-01-02"), 86_400_000.0);
    assert_eq!(
        js::abi::JsDate::utc(1970.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0),
        86_400_000.0
    );
    assert_eq!(
        js::abi::JsDate::from_millis(0.0).to_json(),
        "1970-01-01T00:00:00.000Z"
    );

    let exec_re = js::abi::JsRegExp::new("(b+)", "g").unwrap();
    let matched: js::abi::JsRegExpMatch = exec_re.exec("abbc").unwrap().unwrap();
    assert_eq!(matched.text(), "bb");
    assert_eq!(matched.index(), 1);
    assert_eq!(matched.group(1), Some("bb".to_string()));
    assert_eq!(exec_re.last_index(), 3);

    assert_eq!(
        js::abi::js_string_pad_start_with("5", 3.0, "0").as_deref(),
        Ok("005")
    );
    assert_eq!(
        js::abi::js_string_pad_end_with("5", 3.0, "0").as_deref(),
        Ok("500")
    );
    assert_eq!(js::abi::js_string_repeat("ab", 2.0).unwrap(), "abab");
    assert_eq!(js::abi::js_string_trim_start(" a "), "a ");
    assert_eq!(js::abi::js_string_trim_end(" a "), " a");
    assert_eq!(
        js::abi::js_string_at("abc", -1.0).unwrap().as_deref(),
        Some("c")
    );
    assert_eq!(js::abi::js_string_char_at("abc", 1.0).as_deref(), Ok("b"));
    assert_eq!(js::abi::js_string_char_code_at("abc", 1.0), 98.0);
    assert_eq!(
        js::abi::js_string_code_point_at("😀", 0.0),
        Some(0x1F600 as f64)
    );
    assert_eq!(js::abi::js_string_last_index_of("abc", "b", 2.0), 1);
    assert_eq!(
        js::abi::js_string_substring("abc", 2.0, 0.0).as_deref(),
        Ok("ab")
    );
    assert_eq!(
        js::abi::js_string_substr("abc", 1.0, 1.0).as_deref(),
        Ok("b")
    );
    assert_eq!(
        js::abi::js_string_replace_all("aba", "a", "x").as_deref(),
        Ok("xbx")
    );
    assert_eq!(
        js::abi::js_string_from_char_code(&[65.0, 66.0]).as_deref(),
        Ok("AB")
    );

    let buffer = js::abi::ArrayBuffer::new(4);
    assert_eq!(buffer.byte_length(), 4);
    let mut typed = js::abi::Uint8Array::from_vec(vec![1, 2, 3]);
    typed.set_index(1, 9);
    assert_eq!(typed.get(1), Some(9));
}
