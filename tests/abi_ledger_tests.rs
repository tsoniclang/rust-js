use tsonic_rust_js as js;

#[test]
fn js_backend_legal_abi_paths_are_emit_ready() {
    let mut dense = vec![1_i32, 2_i32];
    assert_eq!(js::abi::array_dense_push(&mut dense, 3), 3);
    assert_eq!(js::abi::array_dense_at(&dense, -1), Some(&3));
    assert_eq!(js::abi::array_dense_map(&dense, |&x| x * 2), vec![2, 4, 6]);
    assert!(js::abi::array_dense_includes(&dense, &2, 0));
    assert_eq!(js::abi::array_dense_index_of(&dense, &3, 0), 2);
    assert_eq!(js::abi::array_dense_join(&dense, ","), "1,2,3");

    let mut out = Vec::new();
    js::abi::console_log_to(&mut out, &[js::abi::JsValue::String("ok".to_string())]).unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "\"ok\"\n");

    let parsed = js::abi::json_parse(r#"{"ok":true}"#).unwrap();
    let text = js::abi::json_stringify(&parsed).unwrap();
    assert_eq!(text, r#"{"ok":true}"#);

    let mut map = js::abi::JsMap::<f64, &str>::new();
    map.set(f64::NAN, "nan");
    assert_eq!(map.get(&f64::NAN), Some(&"nan"));

    let mut set = js::abi::JsSet::<f64>::new();
    set.add(f64::NAN);
    assert!(set.has(&f64::NAN));

    assert_eq!(
        js::abi::JsDate::from_millis(0.0).to_iso_string().unwrap(),
        "1970-01-01T00:00:00.000Z"
    );
    let re = js::abi::JsRegExp::new("a(b+)c", "g").unwrap();
    assert!(re.test("xabbc"));
    assert_eq!(re.find_first("xabbc"), Some((1, 5)));
    assert_eq!(re.replace("abc abbc", "[$1]"), "[b] [bb]");
    assert_eq!(re.search("xabc"), 1);
    assert_eq!(
        js::abi::JsRegExp::new(",", "")
            .unwrap()
            .split("a,b")
            .unwrap(),
        vec!["a", "b"]
    );

    assert_eq!(js::abi::array_dense_find_index(&dense, |&x| x == 2), 1);
    assert_eq!(js::abi::array_dense_find(&dense, |&x| x == 2), Some(2));
    assert_eq!(js::abi::array_dense_find_last(&dense, |&x| x < 3), Some(2));
    assert_eq!(js::abi::array_dense_find_last_index(&dense, |&x| x < 3), 1);
    assert_eq!(
        js::abi::array_dense_flat_one(&[vec![1, 2], vec![3]]),
        vec![1, 2, 3]
    );
    assert_eq!(
        js::abi::array_dense_flat_map_one(&[1, 2], |&x| vec![x, x]),
        vec![1, 1, 2, 2]
    );

    assert_eq!(
        js::abi::json_stringify_with_indent(&parsed, "  ").unwrap(),
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

    let mut exec_re = js::abi::JsRegExp::new("(b+)", "g").unwrap();
    let matched: js::abi::JsRegExpMatch = exec_re.exec("abbc").unwrap();
    assert_eq!(matched.text(), "bb");
    assert_eq!(matched.index(), 1);
    assert_eq!(matched.group(1), Some("bb".to_string()));
    assert_eq!(exec_re.last_index(), 3);

    assert_eq!(js::abi::js_string_pad_start("5", 3, "0"), "005");
    assert_eq!(js::abi::js_string_pad_end("5", 3, "0"), "500");
    assert_eq!(js::abi::js_string_repeat("ab", 2).unwrap(), "abab");
    assert_eq!(js::abi::js_string_trim_start(" a "), "a ");
    assert_eq!(js::abi::js_string_trim_end(" a "), " a");
    assert_eq!(js::abi::js_string_at("abc", -1).as_deref(), Some("c"));
    assert_eq!(js::abi::js_string_char_at("abc", 1), "b");
    assert_eq!(js::abi::js_string_code_point_at("😀", 0), Some(0x1F600));

    let buffer = js::abi::ArrayBuffer::new(4);
    assert_eq!(buffer.byte_length(), 4);
    let mut typed = js::abi::Uint8Array::from_vec(vec![1, 2, 3]);
    typed.set_index(1, 9);
    assert_eq!(typed.get(1), Some(9));
}
