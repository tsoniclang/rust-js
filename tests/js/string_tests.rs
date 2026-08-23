use crate::js;
use tsonic_rust_js::{string, JsArray, JsString};
use tsonic_rust_runtime::JsErrorKind;

#[test]
fn utf16_length_and_indexes() {
    let value = js("abc");
    let emoji = js("😀");

    assert_eq!(string::js_len(&value), 3);
    assert_eq!(string::js_len(&emoji), 2);
    assert_eq!(string::char_at(&value, 5.0), "");
    assert_eq!(string::at(&value, 1.0), Some(js("b")));
    assert_eq!(string::at(&value, -1.0), Some(js("c")));
    assert_eq!(string::at(&value, 9.0), None);
    assert_eq!(string::at(&value, 1.9), Some(js("b")));
    assert_eq!(string::at(&value, f64::NAN), Some(js("a")));
    assert_eq!(string::at(&value, f64::INFINITY), None);
}

#[test]
fn utf16_code_units_and_points() {
    let value = js("abc");
    let emoji = js("😀");

    assert_eq!(string::char_code_at(&emoji, 0.0), 0xD83D as f64);
    assert_eq!(string::char_code_at(&emoji, 1.0), 0xDE00 as f64);
    assert_eq!(string::code_point_at(&emoji, 0.0), Some(0x1F600 as f64));
    assert_eq!(string::code_point_at(&emoji, 1.0), Some(0xDE00 as f64));
    assert_eq!(string::code_point_at(&emoji, -1.0), None);
    assert_eq!(string::char_at(&value, -1.0), "");
    assert!(string::char_code_at(&value, -1.0).is_nan());
    assert_eq!(string::char_at(&emoji, 0.0).units(), &[0xD83D]);
    assert_eq!(string::at(&emoji, 1.0).unwrap().units(), &[0xDE00]);
}

#[test]
fn slice_and_substring_behavior() {
    let javascript = js("javascript");
    let abc = js("abc");

    assert_eq!(string::slice(&javascript, 1.0, Some(3.0)), "av");
    assert_eq!(string::slice(&javascript, -3.0, None), "ipt");
    assert_eq!(string::substring(&abc, 2.9, 0.0), "ab");
    assert_eq!(string::slice(&abc, 2.0, Some(1.0)), "");
    assert_eq!(string::slice(&abc, f64::NAN, Some(f64::INFINITY)), "abc");
    assert_eq!(string::substr(&javascript, 4.9, 6.9), "script");
    assert_eq!(string::substr(&javascript, -6.9, 3.9), "scr");
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
        "he[ll][he][o]o"
    );
    assert_eq!(
        string::replace_all(&js("banana"), &js("a"), &js("$&$&")),
        "baanaanaa"
    );
    assert_eq!(string::replace_all(&js("ab"), &js(""), &js("-")), "-a-b-");
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
    assert_eq!(string::repeat(&js("x"), 3.9).unwrap(), "xxx");
    assert_eq!(string::repeat(&js("x"), f64::NAN).unwrap(), "");
    assert_eq!(
        string::repeat(&js("x"), -1.0).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        string::repeat(&js("x"), f64::INFINITY).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(
        string::repeat(&js("ab"), 8_388_609.0).unwrap_err().kind(),
        JsErrorKind::RangeError
    );
    assert_eq!(string::trim(&js("  hi  ")), "hi");
    assert_eq!(string::trim_start(&js("  hi  ")), "hi  ");
    assert_eq!(string::trim_end(&js("  hi  ")), "  hi");
}

#[test]
fn pad_helpers_and_case() {
    assert_eq!(
        string::pad_start_with(&js("5"), 3.0, &js("0")).unwrap(),
        "005"
    );
    assert_eq!(
        string::pad_end_with(&js("5"), 3.0, &js("0")).unwrap(),
        "500"
    );
    assert_eq!(
        string::pad_start_with(&js("x"), 4.0, &js("ab")).unwrap(),
        "abax"
    );
    assert_eq!(
        string::pad_end_with(&js("x"), 4.0, &js("ab")).unwrap(),
        "xaba"
    );
    assert_eq!(string::pad_start(&js("x"), 3.0).unwrap(), "  x");
    assert_eq!(string::pad_end(&js("x"), 3.0).unwrap(), "x  ");
    assert_eq!(
        string::pad_start_with(&js("x"), -3.9, &js("0")).unwrap(),
        "x"
    );
    assert_eq!(
        string::pad_start_with(&js("x"), f64::NAN, &js("0")).unwrap(),
        "x"
    );
    assert_eq!(
        string::pad_start_with(&js("x"), 3.9, &js("0")).unwrap(),
        "00x"
    );
    assert_eq!(
        string::pad_start_with(&js("x"), f64::INFINITY, &js("")).unwrap(),
        "x"
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
    assert_eq!(string::to_lower_case(&js("AbC")), "abc");
    assert_eq!(string::to_upper_case(&js("AbC")), "ABC");
}

#[test]
fn constructors() {
    assert_eq!(string::from_char_code(&[65.9, 66.0]), "AB");
    assert_eq!(string::from_code_point(&[0x1f600 as f64]).unwrap(), "😀");
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
    assert_eq!(string::to_lower_case(&js("HELLO")), "hello");
    assert_eq!(string::to_upper_case(&js("hello")), "HELLO");
    assert_eq!(string::char_at(&js("😀"), 5.0), "");
    assert_eq!(string::at(&js("😀"), 1.0).unwrap().units(), &[0xDE00]);
    assert_eq!(string::code_point_at(&js("😀"), 0.0), Some(0x1f600 as f64));
    assert_eq!(string::code_point_at(&js("😀"), 1.0), Some(0xDE00 as f64));
    assert_eq!(string::identity(&js("hello")), "hello");

    let b = js("b");
    let c = js("c");
    assert_eq!(string::concat(&js("a"), &[&b, &c]), "abc");
}

#[test]
fn unicode_normalization_and_well_formed_contracts_are_exact() {
    assert_eq!(string::normalize(&js("A\u{030a}")), "\u{00c5}");
    assert_eq!(
        string::normalize_with_form(&js("\u{00c5}"), &js("NFD")).unwrap(),
        "A\u{030a}"
    );
    assert_eq!(
        string::normalize_with_form(&js("\u{fb03}"), &js("NFKC")).unwrap(),
        "ffi"
    );
    assert_eq!(
        string::normalize_with_form(&js("value"), &js("invalid"))
            .unwrap_err()
            .kind(),
        JsErrorKind::TypeError
    );

    let scalar = js("scalar \u{1f600}");
    assert!(string::is_well_formed(&scalar));
    assert_eq!(string::to_well_formed(&scalar), scalar);

    let lone_surrogate = JsString::from_units(vec![0xD800]);
    assert!(!string::is_well_formed(&lone_surrogate));
    assert_eq!(string::to_well_formed(&lone_surrogate).units(), &[0xFFFD]);
}

fn dense(array: JsArray<JsString>) -> Vec<JsString> {
    array.iter_values().collect()
}
