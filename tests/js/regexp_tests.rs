use tsonic_rust_js::regexp::JsRegExp;
use tsonic_rust_runtime::JsErrorKind;

#[test]
fn regexp_test_and_find_first() {
    let re = JsRegExp::new("b+c", "").unwrap();
    assert!(re.test("abbcd"));
    assert!(!re.test("abd"));
    assert_eq!(re.find_first("abbcd"), Some((1, 4)));
    assert_eq!(re.find_first("xyz"), None);
    assert_eq!(re.source(), "b+c");
    assert_eq!(re.flags(), "");

    // Byte offsets over multi-byte input.
    let re = JsRegExp::new("é+", "").unwrap();
    assert_eq!(re.find_first("aéé!"), Some((1, 5)));
}

#[test]
fn regexp_classes_quantifiers_and_alternation() {
    let re = JsRegExp::new(r"[a-c]+", "").unwrap();
    assert_eq!(re.find_first("zzabcaz"), Some((2, 6)));

    let re = JsRegExp::new(r"[^0-9]{2,3}", "").unwrap();
    assert_eq!(re.find_first("12abcd3"), Some((2, 5)));

    let re = JsRegExp::new(r"\d{2}|\w+", "").unwrap();
    assert_eq!(re.find_first("!!42go"), Some((2, 4)));

    let re = JsRegExp::new(r"a{2,}", "").unwrap();
    assert!(re.test("caaab"));
    assert!(!re.test("cab"));
}

#[test]
fn regexp_anchors_and_flags() {
    let re = JsRegExp::new("^b$", "m").unwrap();
    assert!(re.test("a\nb\nc"));
    assert!(!JsRegExp::new("^b$", "").unwrap().test("a\nb\nc"));

    let re = JsRegExp::new("abc", "i").unwrap();
    assert!(re.test("xAbCy"));
    assert_eq!(re.flags(), "i");
}

#[test]
fn regexp_replace_with_group_substitution() {
    let re = JsRegExp::new(r"(\w+)@(\w+)", "").unwrap();
    assert_eq!(re.replace("mail: a@b, c@d", "$2:$1"), "mail: b:a, c@d");

    let global = JsRegExp::new(r"(\w+)@(\w+)", "g").unwrap();
    assert_eq!(global.replace("a@b c@d", "[$&]"), "[a@b] [c@d]");
    assert_eq!(global.replace("a@b", "$$ $` $'"), "$  ");

    // Unknown group references stay literal.
    let re = JsRegExp::new("a", "").unwrap();
    assert_eq!(re.replace("a", "$1$0x"), "$1$0x");
}

#[test]
fn regexp_split_and_search() {
    let re = JsRegExp::new(r"\s*,\s*", "").unwrap();
    assert_eq!(re.split("a , b,c").unwrap(), vec!["a", "b", "c"]);
    assert_eq!(JsRegExp::new("x", "").unwrap().split("").unwrap(), vec![""]);
    assert_eq!(
        JsRegExp::new("", "").unwrap().split("ab").unwrap(),
        vec!["a", "b"]
    );
    assert_eq!(
        JsRegExp::new("(a)", "")
            .unwrap()
            .split("ab")
            .unwrap_err()
            .kind(),
        JsErrorKind::Unsupported
    );

    assert_eq!(JsRegExp::new("b", "").unwrap().search("ab"), 1);
    assert_eq!(JsRegExp::new("z", "").unwrap().search("ab"), -1);
    // search reports UTF-16 code-unit indexes: 😀 is two code units.
    assert_eq!(JsRegExp::new("b", "").unwrap().search("😀b"), 2);
}

#[test]
fn regexp_rejects_constructs_outside_the_subset() {
    let cases: &[(&str, &str)] = &[
        (r"a*?", ""),
        (r"a+?", ""),
        (r"a??", ""),
        (r"a{1,2}?", ""),
        (r"(a)\1", ""),
        (r"(?=a)", ""),
        (r"(?!a)", ""),
        (r"(?<=a)", ""),
        (r"(?<!a)", ""),
        (r"(?<name>a)", ""),
        (r"\bword", ""),
        (r"\Bword", ""),
        (r"\p{L}", ""),
        (r"\k<name>", ""),
        (r"\cA", ""),
        (r"a{3,1}", ""),
        (r"a{9999}", ""),
        (r"(a", ""),
        (r"a)", ""),
        (r"[a", ""),
        (r"a\", ""),
        (r"*a", ""),
        ("a", "u"),
        ("a", "y"),
        ("a", "s"),
        ("a", "d"),
        ("a", "gg"),
    ];
    for (pattern, flags) in cases {
        let err = JsRegExp::new(pattern, flags).unwrap_err();
        assert_eq!(
            err.kind(),
            JsErrorKind::SyntaxError,
            "pattern {pattern:?} flags {flags:?}"
        );
    }
}

#[test]
fn regexp_empty_match_iteration_terminates() {
    let re = JsRegExp::new("a*", "g").unwrap();
    assert_eq!(re.replace("bab", "-"), "-b--b-");
    assert_eq!(re.replace("", "x"), "x");

    // Loops with possibly-empty bodies must not spin forever.
    assert!(JsRegExp::new("(?:a|)*b", "").unwrap().test("b"));
    assert!(JsRegExp::new("(?:a|){3}b", "").unwrap().test("ab"));
}

#[test]
fn regexp_flag_and_last_index_getters() {
    let mut re = JsRegExp::new("a", "gim").unwrap();
    assert!(re.global());
    assert!(re.ignore_case());
    assert!(re.multiline());
    assert_eq!(re.last_index(), 0);
    re.set_last_index(3);
    assert_eq!(re.last_index(), 3);

    let re = JsRegExp::new("a", "").unwrap();
    assert!(!re.global());
    assert!(!re.ignore_case());
    assert!(!re.multiline());
}

#[test]
fn regexp_exec_advances_and_resets_last_index() {
    let mut re = JsRegExp::new(r"\d+", "g").unwrap();
    let first = re.exec("a1b22c333").unwrap();
    assert_eq!(first.text(), "1");
    assert_eq!(first.index(), 1);
    assert_eq!(first.input(), "a1b22c333");
    assert_eq!(re.last_index(), 2);

    let second = re.exec("a1b22c333").unwrap();
    assert_eq!((second.text(), second.index()), ("22".to_string(), 3));
    assert_eq!(re.last_index(), 5);

    let third = re.exec("a1b22c333").unwrap();
    assert_eq!((third.text(), third.index()), ("333".to_string(), 6));
    assert_eq!(re.last_index(), 9);

    // Exhausted: null result resets lastIndex to 0, then matching restarts.
    assert!(re.exec("a1b22c333").is_none());
    assert_eq!(re.last_index(), 0);
    assert_eq!(re.exec("a1b22c333").unwrap().text(), "1");

    // lastIndex beyond the input: no match, reset to 0.
    re.set_last_index(99);
    assert!(re.exec("a1").is_none());
    assert_eq!(re.last_index(), 0);

    // Negative lastIndex behaves like 0 (ToLength clamp).
    re.set_last_index(-5);
    assert_eq!(re.exec("a1").unwrap().index(), 1);
}

#[test]
fn regexp_exec_without_g_ignores_state() {
    let mut re = JsRegExp::new("o", "").unwrap();
    re.set_last_index(2);
    let m = re.exec("foo").unwrap();
    assert_eq!((m.text(), m.index()), ("o".to_string(), 1));
    // Non-global exec never touches lastIndex.
    assert_eq!(re.last_index(), 2);
    assert_eq!(re.exec("foo").unwrap().index(), 1);
}

#[test]
fn regexp_exec_reports_utf16_indexes_and_last_index() {
    let mut re = JsRegExp::new(r"\d", "g").unwrap();
    let m = re.exec("你1好2").unwrap();
    assert_eq!((m.text(), m.index()), ("1".to_string(), 1));
    assert_eq!(re.last_index(), 2);
    let m = re.exec("你1好2").unwrap();
    assert_eq!((m.text(), m.index()), ("2".to_string(), 3));
}

#[test]
fn regexp_match_carrier_exposes_groups() {
    let re = JsRegExp::new(r"(\w+)@(\w+)|(!)", "").unwrap();
    let m = re.match_first("mail: a@b").unwrap();
    assert_eq!(m.text(), "a@b");
    assert_eq!(m.index(), 6);
    assert_eq!(m.input(), "mail: a@b");
    assert_eq!(m.group_count(), 3);
    assert_eq!(m.group(0), Some("a@b".to_string()));
    assert_eq!(m.group(1), Some("a".to_string()));
    assert_eq!(m.group(2), Some("b".to_string()));
    // Unmatched alternation branch and out-of-range groups are None.
    assert_eq!(m.group(3), None);
    assert_eq!(m.group(4), None);

    assert!(re.match_first("no matches here?").is_none());
}

#[test]
fn regexp_match_strings_collects_all_texts_or_none() {
    let re = JsRegExp::new(r"\d+", "g").unwrap();
    assert_eq!(
        re.match_strings("a1b22c333").unwrap(),
        vec!["1", "22", "333"]
    );
    assert!(re.match_strings("abc").is_none());

    // Empty matches advance without spinning (JS: "baab".match(/a*/g)).
    let re = JsRegExp::new("a*", "g").unwrap();
    assert_eq!(re.match_strings("baab").unwrap(), vec!["", "aa", "", ""]);
}

#[test]
fn regexp_match_all_requires_the_g_flag() {
    let err = JsRegExp::new(r"\d", "")
        .unwrap()
        .match_all("a1")
        .unwrap_err();
    assert_eq!(err.kind(), JsErrorKind::TypeError);

    let re = JsRegExp::new(r"(\w)(\d)?", "g").unwrap();
    let matches = re.match_all("a1 b").unwrap();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].text(), "a1");
    assert_eq!(matches[0].group(1), Some("a".to_string()));
    assert_eq!(matches[0].group(2), Some("1".to_string()));
    assert_eq!(matches[1].text(), "b");
    assert_eq!(matches[1].group(2), None);
    assert_eq!(matches[1].index(), 3);

    // matchAll is stateless: lastIndex is not consulted or mutated.
    let mut re = JsRegExp::new("a", "g").unwrap();
    re.set_last_index(2);
    assert_eq!(re.match_all("aaa").unwrap().len(), 3);
    assert_eq!(re.last_index(), 2);
}
