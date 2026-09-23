use crate::js;
use tsonic_rust_js::{exact_string as string, string as native_string, JsArray, JsString};
use tsonic_rust_runtime::JsErrorKind;

#[test]
fn native_string_iteration_is_lazy_and_scalar_exact() {
    let mut values = native_string::NativeStringIterator::new("aé😀z".to_owned());
    assert_eq!(values.next().as_deref(), Some("a"));
    assert_eq!(values.next().as_deref(), Some("é"));
    assert_eq!(values.next().as_deref(), Some("😀"));
    assert_eq!(values.next().as_deref(), Some("z"));
    assert_eq!(values.next(), None);
    assert_eq!(values.next(), None);
    assert_eq!(
        native_string::NativeStringIterator::new(String::new()).next(),
        None
    );
    assert_eq!(
        native_string::NativeStringIterator::new("abc".repeat(100_000))
            .take(1)
            .collect::<Vec<_>>(),
        ["a"]
    );
}

#[test]
fn utf16_length_and_indexes() {
    let value = js("abc");
    let emoji = js("😀");

    assert_eq!(string::js_len(&value), 3);
    assert_eq!(string::js_len(&emoji), 2);
    assert_eq!(string::char_at(&value, 5.0), js(""));
    assert_eq!(string::at(&value, 1.0), Some(js("b")));
    assert_eq!(string::at(&value, -1.0), Some(js("c")));
    assert_eq!(string::at(&value, 9.0), None);
    assert_eq!(string::at(&value, 1.9), Some(js("b")));
    assert_eq!(string::at(&value, f64::NAN), Some(js("a")));
    assert_eq!(string::at(&value, f64::INFINITY), None);
}

#[test]
fn native_strings_remain_default_and_exact_utf16_requires_the_explicit_carrier() {
    assert_eq!(native_string::js_len("A😀"), 5);
    assert_eq!(native_string::char_at("plain", 1.0).unwrap(), "l");
    assert_eq!(
        native_string::char_at("😀", 1.0).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(native_string::char_at("😀", 0.0).unwrap(), "😀");

    let exact = JsString::from_utf8("😀");
    assert_eq!(string::char_at(&exact, 0.0).units(), &[0xD83D]);
}

#[test]
fn native_string_indexes_and_searches_use_utf8_bytes() {
    let text = "aé😀z";
    assert_eq!(native_string::js_len(text), 8);
    assert_eq!(native_string::char_code_at(text, 1.0), 233.0);
    assert_eq!(native_string::code_point_at(text, 3.0), Some(128512));
    assert!(native_string::char_code_at(text, 2.0).is_nan());
    assert_eq!(native_string::code_point_at(text, 4.0), None);
    assert_eq!(native_string::slice(text, 1.0, Some(7.0)).unwrap(), "é😀");
    assert_eq!(
        native_string::slice(text, 2.0, Some(7.0))
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(native_string::index_of(text, "😀", 0.0), 3);
    assert_eq!(native_string::index_of(text, "z", 4.0), 7);
    assert_eq!(native_string::last_index_of(text, "😀", 4.0), 3);
    assert_eq!(native_string::last_index_of(text, "😀", 2.0), -1);
    assert!(native_string::starts_with_from_start(text, "aé"));
    assert!(native_string::ends_with_at_end(text, "😀z"));
    assert!(native_string::includes_from_start(text, "é😀"));
    assert!(!native_string::starts_with(text, "😀", 4.0));
    assert!(!native_string::ends_with(text, "é", 2.0));
    assert_eq!(native_string::at(text, -1.0).unwrap().as_deref(), Some("z"));
}

#[test]
fn native_string_construction_and_padding_preserve_valid_utf8() {
    assert_eq!(
        native_string::from_char_code(&[233.0, 128512.0]).unwrap(),
        "é😀"
    );
    assert!(native_string::from_code_point(&[0xd800 as f64]).is_err());
    assert!(native_string::from_char_code(&[65.5]).is_err());
    assert_eq!(
        native_string::split_all("a😀é", "")
            .unwrap()
            .iter_values()
            .collect::<Vec<_>>(),
        ["a", "😀", "é"]
    );
    assert_eq!(native_string::replace_all("😀", "", "-").unwrap(), "-😀-");
    assert_eq!(
        native_string::pad_start_with("😀", 6.0, "é").unwrap(),
        "é😀"
    );
    assert_eq!(native_string::pad_end_with("😀", 6.0, "é").unwrap(), "😀é");
    assert_eq!(
        native_string::pad_end_with("😀", 5.0, "é")
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        native_string::trim("\u{feff}x\u{feff}"),
        "\u{feff}x\u{feff}"
    );
    assert_eq!(native_string::trim("\u{85}x\u{85}"), "x");
}

#[test]
fn utf16_code_units_and_points() {
    let value = js("abc");
    let emoji = js("😀");

    assert_eq!(string::char_code_at(&emoji, 0.0), 0xD83D as f64);
    assert_eq!(string::char_code_at(&emoji, 1.0), 0xDE00 as f64);
    assert_eq!(string::code_point_at(&emoji, 0.0), Some(0x1F600));
    assert_eq!(string::code_point_at(&emoji, 1.0), Some(0xDE00));
    assert_eq!(string::code_point_at(&emoji, -1.0), None);
    assert_eq!(string::char_at(&value, -1.0), js(""));
    assert!(string::char_code_at(&value, -1.0).is_nan());
    assert_eq!(string::char_at(&emoji, 0.0).units(), &[0xD83D]);
    assert_eq!(string::at(&emoji, 1.0).unwrap().units(), &[0xDE00]);
}

#[test]
fn slice_and_substring_behavior() {
    let javascript = js("javascript");
    let abc = js("abc");

    assert_eq!(string::slice(&javascript, 1.0, Some(3.0)), js("av"));
    assert_eq!(string::slice(&javascript, -3.0, None), js("ipt"));
    assert_eq!(string::substring(&abc, 2.9, 0.0), js("ab"));
    assert_eq!(string::slice(&abc, 2.0, Some(1.0)), js(""));
    assert_eq!(
        string::slice(&abc, f64::NAN, Some(f64::INFINITY)),
        js("abc")
    );
    assert_eq!(string::substr(&javascript, 4.9, 6.9), js("script"));
    assert_eq!(string::substr(&javascript, -6.9, 3.9), js("scr"));
}

#[test]
fn search_and_replace() {
    let array = js("array");

    assert!(string::includes(&array, &js("ra"), 0.0));
    assert!(!string::includes(&array, &js("RA"), 0.0));
    assert!(string::starts_with(&array, &js("ar"), 0.0));
    assert!(string::starts_with_from_start(&array, &js("ar")));
    assert!(string::starts_with(&array, &js("ar"), -5.0));
    assert!(string::starts_with(&array, &js(""), 30.0));
    assert!(string::ends_with_at_end(&array, &js("ay")));
    assert_eq!(
        string::replace(&js("hello"), &js("ll"), &js("[$&][$`][$']")),
        js("he[ll][he][o]o")
    );
    assert_eq!(
        string::replace_all(&js("banana"), &js("a"), &js("$&$&")),
        js("baanaanaa")
    );
    assert_eq!(
        string::replace_all(&js("ab"), &js(""), &js("-")),
        js("-a-b-")
    );
}

#[test]
fn split_and_repeat_and_trim() {
    assert_eq!(
        dense(string::split_all(&js("a,b,c"), &js(","))),
        vec![js("a"), js("b"), js("c")]
    );
    assert_eq!(
        dense(string::split(&js("abc"), &js(""), 2.9)),
        vec![js("a"), js("b")]
    );
    assert_eq!(
        dense(string::split_all(&js("a,,c"), &js(","))),
        vec![js("a"), js(""), js("c")]
    );
    assert!(dense(string::split(&js("a,b"), &js(","), f64::NAN)).is_empty());
    assert_eq!(string::repeat(&js("x"), 3.9).unwrap(), js("xxx"));
    assert_eq!(string::repeat(&js("x"), f64::NAN).unwrap(), js(""));
    assert_eq!(
        string::repeat(&js("x"), -1.0).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        string::repeat(&js("x"), f64::INFINITY).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        string::repeat(&js("ab"), f64::MAX).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(string::trim(&js("  hi  ")), js("hi"));
    assert_eq!(string::trim_start(&js("  hi  ")), js("hi  "));
    assert_eq!(string::trim_end(&js("  hi  ")), js("  hi"));
}

#[test]
fn native_and_exact_strings_are_not_limited_to_sixteen_megacodeunits() {
    let length = 16_777_217;
    let native = native_string::repeat("x", length as f64).unwrap();
    assert_eq!(native.len(), length);
    assert!(native.bytes().all(|byte| byte == b'x'));
    let exact = string::repeat(&js("x"), length as f64).unwrap();
    assert_eq!(exact.len(), length);
    assert!(exact.units().iter().all(|unit| *unit == u16::from(b'x')));
    let native_padded = native_string::pad_start_with("tail", length as f64, "ab").unwrap();
    assert_eq!(native_padded.len(), length);
    assert!(native_padded.starts_with("ababa"));
    assert!(native_padded.ends_with("tail"));
    let exact_padded = string::pad_end_with(&js("head"), length as f64, &js("ab")).unwrap();
    assert_eq!(exact_padded.len(), length);
    assert_eq!(&exact_padded.units()[..6], &[104, 101, 97, 100, 97, 98]);
    assert_eq!(exact_padded.units()[length - 1], 97);
}

#[test]
fn string_capacity_preserves_empty_results_and_checks_real_overflows() {
    assert_eq!(string::repeat(&js(""), f64::MAX).unwrap(), js(""));
    assert_eq!(native_string::repeat("", f64::MAX).unwrap(), "");
    for count in [-1.0, f64::NEG_INFINITY, f64::INFINITY, f64::MAX] {
        assert_eq!(
            string::repeat(&js("😀"), count).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
        assert_eq!(
            native_string::repeat("😀", count).unwrap_err().kind(),
            JsErrorKind::RangeError
        );
    }
    for count in [f64::NAN, -0.9, 0.0] {
        assert_eq!(string::repeat(&js("😀"), count).unwrap(), js(""));
        assert_eq!(native_string::repeat("😀", count).unwrap(), "");
    }
    assert_eq!(native_string::repeat("a😀", 3.9).unwrap(), "a😀a😀a😀");
    assert_eq!(string::repeat(&js("a😀"), 3.9).unwrap(), js("a😀a😀a😀"));
    assert_eq!(
        native_string::pad_start_with("x", f64::INFINITY, "").unwrap(),
        "x"
    );
    assert_eq!(
        string::pad_end_with(&js("x"), f64::INFINITY, &js("")).unwrap(),
        js("x")
    );
}

#[test]
fn pad_helpers_and_case() {
    assert_eq!(
        string::pad_start_with(&js("5"), 3.0, &js("0")).unwrap(),
        js("005")
    );
    assert_eq!(
        string::pad_end_with(&js("5"), 3.0, &js("0")).unwrap(),
        js("500")
    );
    assert_eq!(
        string::pad_start_with(&js("x"), 4.0, &js("ab")).unwrap(),
        js("abax")
    );
    assert_eq!(
        string::pad_end_with(&js("x"), 4.0, &js("ab")).unwrap(),
        js("xaba")
    );
    assert_eq!(string::pad_start(&js("x"), 3.0).unwrap(), js("  x"));
    assert_eq!(string::pad_end(&js("x"), 3.0).unwrap(), js("x  "));
    assert_eq!(
        string::pad_start_with(&js("x"), -3.9, &js("0")).unwrap(),
        js("x")
    );
    assert_eq!(
        string::pad_start_with(&js("x"), f64::NAN, &js("0")).unwrap(),
        js("x")
    );
    assert_eq!(
        string::pad_start_with(&js("x"), 3.9, &js("0")).unwrap(),
        js("00x")
    );
    assert_eq!(
        string::pad_start_with(&js("x"), f64::INFINITY, &js("")).unwrap(),
        js("x")
    );
    assert_eq!(
        string::pad_start(&js("x"), f64::INFINITY)
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        string::pad_start_with(&js("x"), 2.0, &js("😀"))
            .unwrap()
            .units(),
        &[0xD83D, b'x' as u16]
    );
    assert_eq!(string::to_lower_case(&js("AbC")), js("abc"));
    assert_eq!(string::to_upper_case(&js("AbC")), js("ABC"));
}

#[test]
fn constructors() {
    assert_eq!(string::from_char_code(&[65.9, 66.0]), js("AB"));
    assert_eq!(
        string::from_code_point(&[0x1f600 as f64]).unwrap(),
        js("😀")
    );
    assert_eq!(
        string::from_code_point(&[0xD800 as f64]).unwrap().units(),
        &[0xD800]
    );
    assert_eq!(
        string::from_code_point(&[0x11_0000 as f64])
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(string::from_char_code(&[0xD800 as f64]).units(), &[0xD800]);
}

#[test]
fn search_edge_cases() {
    assert_eq!(string::index_of(&js("abc"), &js(""), 10.0), 3);
    assert_eq!(string::index_of(&js("abc"), &js("z"), -10.0), -1);
    assert_eq!(string::last_index_of(&js("banana"), &js("ana"), -1.0), -1);
    assert_eq!(string::last_index_of(&js("banana"), &js("ana"), -10.0), -1);
    assert_eq!(string::last_index_of(&js("abc"), &js(""), 1.9), 1);
    assert!(string::includes(&js(""), &js(""), 0.0));
    assert!(!string::includes(&js("a"), &js("a"), 1.0));
}

#[test]
fn conversion_helpers() {
    assert_eq!(string::to_lower_case(&js("HELLO")), js("hello"));
    assert_eq!(string::to_upper_case(&js("hello")), js("HELLO"));
    assert_eq!(string::char_at(&js("😀"), 5.0), js(""));
    assert_eq!(string::at(&js("😀"), 1.0).unwrap().units(), &[0xDE00]);
    assert_eq!(string::code_point_at(&js("😀"), 0.0), Some(0x1f600));
    assert_eq!(string::code_point_at(&js("😀"), 1.0), Some(0xDE00));
    assert_eq!(string::identity(&js("hello")), js("hello"));

    let b = js("b");
    let c = js("c");
    assert_eq!(string::concat(&js("a"), &[&b, &c]), js("abc"));
    assert_eq!(js("a").concat_values([b, c]), js("abc"));
}

#[test]
fn unicode_normalization_and_well_formed_contracts_are_exact() {
    assert_eq!(string::normalize(&js("A\u{030a}")), js("\u{00c5}"));
    assert_eq!(
        string::normalize_with_form(&js("\u{00c5}"), "NFD").unwrap(),
        js("A\u{030a}")
    );
    assert_eq!(
        string::normalize_with_form(&js("\u{fb03}"), "NFKC").unwrap(),
        js("ffi")
    );
    assert_eq!(
        string::normalize_with_form(&js("value"), "invalid")
            .unwrap_err()
            .kind(),
        JsErrorKind::TypeError
    );

    let scalar = js("scalar \u{1f600}");
    assert!(string::is_well_formed(&scalar));
    assert_eq!(string::to_well_formed(&scalar), "scalar \u{1f600}");

    let lone_surrogate = JsString::from_units(vec![0xD800]);
    assert!(!string::is_well_formed(&lone_surrogate));
    assert_eq!(string::to_well_formed(&lone_surrogate), "\u{FFFD}");
}

fn dense(array: JsArray<JsString>) -> Vec<JsString> {
    array.iter_values().collect()
}
