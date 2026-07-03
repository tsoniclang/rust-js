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
