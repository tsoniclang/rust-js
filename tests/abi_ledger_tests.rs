use tsonic_rust_js as js;

fn text(value: &str) -> String {
    value.to_owned()
}

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
    assert_eq!(dense.join(&text(",")), "1,2,3");
    assert_eq!(dense.slice(1.0, None).values(), vec![Some(2), Some(3)]);
    assert_eq!(dense.slice_to(0.0, 2.0).values(), vec![Some(1), Some(2)]);
    assert!(js::abi::number_is_finite(1.0));
    assert!(js::abi::number_is_integer(1.0));
    assert!(!js::abi::number_is_nan(1.0));
    assert!(js::abi::number_is_safe_integer(1.0));

    let mut out = Vec::new();
    js::abi::console_log_to(
        &mut out,
        &[js::abi::JsValue::String(js::abi::JsString::from_utf8("ok"))],
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "ok\n");
    let source_text = text("kept");
    let converted = js::abi::js_value_from_string(&source_text);
    let cloned = js::abi::clone_js_value(&converted);
    assert_eq!(source_text, "kept");
    assert_eq!(converted, cloned);

    let parsed = js::abi::json_parse(&text(r#"{"ok":true}"#)).unwrap();
    let serialized = js::abi::json_stringify(&parsed).unwrap().unwrap();
    assert_eq!(serialized, r#"{"ok":true}"#);

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
    let re = js::abi::regexp_new_native("a(b+)c", "g").unwrap();
    assert!(js::abi::regexp_test_native(&re, "xabbc").unwrap());
    re.set_last_index(0.0);
    let first = js::abi::regexp_exec_native(&re, "xabbc").unwrap().unwrap();
    assert_eq!(
        (first.index(), first.index() + first.text().len() as f64),
        (1.0, 5.0)
    );
    re.set_last_index(0.0);
    assert_eq!(
        js::abi::regexp_replace_native(&re, "abc abbc", "[$1]").unwrap(),
        "[b] [bb]"
    );
    assert_eq!(js::abi::regexp_search_native(&re, "xabc").unwrap(), 1.0);
    assert_eq!(
        js::abi::regexp_split_native(&js::abi::regexp_new_native(",", "").unwrap(), "a,b", None,)
            .unwrap()
            .iter_values()
            .collect::<Vec<_>>(),
        vec!["a".to_owned(), "b".to_owned()]
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

    assert_eq!(js::abi::JsDate::parse(&text("1970-01-02")), 86_400_000.0);
    assert_eq!(
        js::abi::JsDate::utc(1970.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0),
        86_400_000.0
    );
    assert_eq!(
        js::abi::JsDate::from_millis(0.0).to_json(),
        Some("1970-01-01T00:00:00.000Z".to_string())
    );

    let exec_re = js::abi::regexp_new_native("(b+)", "g").unwrap();
    let matched: js::abi::RegExpExecArray = js::abi::regexp_exec_native(&exec_re, "abbc")
        .unwrap()
        .unwrap();
    assert_eq!(matched.text(), "bb");
    assert_eq!(matched.index(), 1.0);
    assert_eq!(matched.group(1), Some(text("bb")));
    assert_eq!(exec_re.last_index(), 3.0);

    assert_eq!(
        js::abi::js_string_pad_start_with(&text("5"), 3.0, &text("0")).unwrap(),
        "005"
    );
    assert_eq!(
        js::abi::js_string_pad_end_with(&text("5"), 3.0, &text("0")).unwrap(),
        "500"
    );
    assert_eq!(js::abi::js_string_repeat(&text("ab"), 2.0).unwrap(), "abab");
    assert_eq!(js::abi::js_string_trim_start(&text(" a ")), "a ");
    assert_eq!(js::abi::js_string_trim_end(&text(" a ")), " a");
    assert_eq!(
        js::abi::js_string_at(&text("abc"), -1.0).unwrap(),
        Some(text("c"))
    );
    assert_eq!(js::abi::js_string_char_at(&text("abc"), 1.0).unwrap(), "b");
    assert_eq!(js::abi::js_string_char_code_at(&text("abc"), 1.0), 98.0);
    assert_eq!(
        js::abi::js_string_code_point_at(&text("😀"), 0.0),
        Some(0x1F600 as f64)
    );
    assert_eq!(
        js::abi::js_string_last_index_of(&text("abc"), &text("b"), 2.0),
        1
    );
    assert_eq!(
        js::abi::js_string_substring(&text("abc"), 2.0, 0.0).unwrap(),
        "ab"
    );
    assert_eq!(
        js::abi::js_string_substr(&text("abc"), 1.0, 1.0).unwrap(),
        "b"
    );
    assert_eq!(
        js::abi::js_string_replace_all(&text("aba"), &text("a"), &text("x")).unwrap(),
        "xbx"
    );
    assert_eq!(
        js::abi::js_string_from_char_code(&[65.0, 66.0]).unwrap(),
        "AB"
    );

    let exact = js::abi::js_string_from_utf8("😀".to_owned());
    assert_eq!(exact.units(), &[0xD83D, 0xDE00]);
    assert_eq!(
        js::abi::js_value_from_exact_string(&exact),
        js::abi::JsValue::String(exact)
    );

    let buffer = js::abi::ArrayBuffer::new(4.0).unwrap();
    assert_eq!(buffer.byte_length(), 4.0);
    let typed = js::abi::Uint8Array::from_vec(vec![1.0, 2.0, 3.0]).unwrap();
    typed.set_number(1.0, 9.0);
    assert_eq!(typed.get_number(1.0), Some(9.0));
}
