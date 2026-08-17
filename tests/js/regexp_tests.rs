use tsonic_rust_js::regexp::JsRegExp;
use tsonic_rust_runtime::JsErrorKind;

fn dense_values<T: Clone>(array: &tsonic_rust_js::JsArray<T>) -> Vec<T> {
    array.values().into_iter().flatten().collect()
}

#[test]
fn regexp_clones_preserve_object_identity_and_shared_last_index() {
    let regexp = JsRegExp::new(r"\d+", "g").unwrap();
    let alias = regexp.clone();
    let distinct = JsRegExp::new(r"\d+", "g").unwrap();

    assert_eq!(regexp, alias);
    assert_ne!(regexp, distinct);
    assert!(alias.test("a1b22").unwrap());
    assert_eq!(regexp.last_index(), 2);
    assert!(regexp.test("a1b22").unwrap());
    assert_eq!(alias.last_index(), 5);
}

#[test]
fn regexp_test_and_find_first() {
    let re = JsRegExp::new("b+c", "").unwrap();
    assert!(re.test("abbcd").unwrap());
    assert!(!re.test("abd").unwrap());
    assert_eq!(re.find_first("abbcd").unwrap(), Some((1, 4)));
    assert_eq!(re.find_first("xyz").unwrap(), None);
    assert_eq!(re.source(), "b+c");
    assert_eq!(re.flags(), "");

    // Byte offsets over multi-byte input.
    let re = JsRegExp::new("é+", "").unwrap();
    assert_eq!(re.find_first("aéé!").unwrap(), Some((1, 5)));
}

#[test]
fn regexp_classes_quantifiers_and_alternation() {
    let re = JsRegExp::new(r"[a-c]+", "").unwrap();
    assert_eq!(re.find_first("zzabcaz").unwrap(), Some((2, 6)));

    let re = JsRegExp::new(r"[a-z]{2,3}", "").unwrap();
    assert_eq!(re.find_first("12abcd3").unwrap(), Some((2, 5)));

    let re = JsRegExp::new(r"\d{2}|\w+", "").unwrap();
    assert_eq!(re.find_first("!!42go").unwrap(), Some((2, 4)));

    let re = JsRegExp::new(r"a{2,}", "").unwrap();
    assert!(re.test("caaab").unwrap());
    assert!(!re.test("cab").unwrap());
}

#[test]
fn regexp_anchors_and_flags() {
    let re = JsRegExp::new("^b$", "m").unwrap();
    assert!(re.test("a\nb\nc").unwrap());
    assert!(!JsRegExp::new("^b$", "").unwrap().test("a\nb\nc").unwrap());

    let re = JsRegExp::new("abc", "i").unwrap();
    assert!(re.test("xAbCy").unwrap());
    assert_eq!(re.flags(), "i");
}

#[test]
fn regexp_replace_with_group_substitution() {
    let re = JsRegExp::new(r"(\w+)@(\w+)", "").unwrap();
    assert_eq!(
        re.replace("mail: a@b, c@d", "$2:$1").unwrap(),
        "mail: b:a, c@d"
    );

    let global = JsRegExp::new(r"(\w+)@(\w+)", "g").unwrap();
    assert_eq!(global.replace("a@b c@d", "[$&]").unwrap(), "[a@b] [c@d]");
    assert_eq!(global.replace("a@b", "$$ $` $'").unwrap(), "$  ");

    // Unknown group references stay literal.
    let re = JsRegExp::new("a", "").unwrap();
    assert_eq!(re.replace("a", "$1$0x").unwrap(), "$1$0x");
}

#[test]
fn regexp_split_and_search() {
    let re = JsRegExp::new(r"\s*,\s*", "").unwrap();
    assert_eq!(
        dense_values(&re.split("a , b,c").unwrap()),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        dense_values(&JsRegExp::new("x", "").unwrap().split("").unwrap()),
        vec![""]
    );
    assert_eq!(
        dense_values(&JsRegExp::new("", "").unwrap().split("ab").unwrap()),
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

    assert_eq!(JsRegExp::new("b", "").unwrap().search("ab").unwrap(), 1);
    assert_eq!(JsRegExp::new("z", "").unwrap().search("ab").unwrap(), -1);
    // search reports UTF-16 code-unit indexes: 😀 is two code units.
    assert_eq!(JsRegExp::new("b", "").unwrap().search("😀b").unwrap(), 2);
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
fn regexp_rejects_code_unit_sensitive_constructs_at_construction() {
    // `.`, negated classes, and classes reaching surrogate/astral code
    // points match lone surrogates in Node's non-`u` mode (e.g.
    // /./.exec("😀") yields the high surrogate alone) — unrepresentable in
    // a Rust String — so they are rejected fail-closed at construction,
    // independent of any input searched.
    let cases: &[(&str, &str)] = &[
        (r".", ""),
        (r"a.c", ""),
        (r".*", "g"),
        (r"[^a]", ""),
        (r"[^a-z0-9]", ""),
        (r"[^]", ""),
        (r"\D", ""),
        (r"\W", ""),
        (r"\S", ""),
        (r"[\D]", ""),
        (r"[\W]", ""),
        (r"[\S]", ""),
        (r"[\x00-￿]", ""), // range covers surrogate code points
        (r"[퟿-]", ""),    // crosses the surrogate gap
        (r"[a-]", ""),    // upper bound past U+D7FF
        (r"[😀]", ""),     // astral class member = surrogate pair members in Node
        (r"[😀-😁]", ""),  // astral range
        (r"😀+", ""),      // quantifier binds to the low surrogate in Node
        (r"a😀?b", ""),
        (r"😀{2}", ""),
    ];
    for (pattern, flags) in cases {
        let err = JsRegExp::new(pattern, flags).unwrap_err();
        assert_eq!(
            err.kind(),
            JsErrorKind::SyntaxError,
            "pattern {pattern:?} flags {flags:?}"
        );
        assert!(
            err.message().contains("outside the oracle-proven subset"),
            "pattern {pattern:?}: unexpected message {:?}",
            err.message()
        );
    }

    // The retained boundary: positive classes with ranges up to U+D7FF (and
    // BMP singles above the surrogate gap) stay accepted and exact.
    assert!(JsRegExp::new(r"[\x00-퟿]", "").is_ok());
    assert!(JsRegExp::new(r"[a-z你￿]", "").is_ok());
    let boundary = JsRegExp::new(r"[\x61-퟿]+", "").unwrap();
    assert!(!boundary.test("😀").unwrap()); // Node: neither surrogate half is in range
    assert!(boundary.test("aЖ你").unwrap());

    // A grouped astral literal repeats the whole surrogate pair in Node too,
    // so it stays accepted and exact (unlike a directly quantified one).
    let grouped = JsRegExp::new(r"(?:💚)+", "").unwrap();
    assert!(grouped.test("a💚💚b").unwrap());
}

#[test]
fn regexp_empty_match_iteration_terminates() {
    let re = JsRegExp::new("a*", "g").unwrap();
    assert_eq!(re.replace("bab", "-").unwrap(), "-b--b-");
    assert_eq!(re.replace("", "x").unwrap(), "x");

    // Loops with possibly-empty bodies must not spin forever.
    assert!(JsRegExp::new("(?:a|)*b", "").unwrap().test("b").unwrap());
    assert!(JsRegExp::new("(?:a|){3}b", "").unwrap().test("ab").unwrap());
}

#[test]
fn regexp_backtracking_exhaustion_fails_deterministically() {
    let regexp = JsRegExp::new("^(a+)+b$", "").unwrap();
    let error = regexp.test(&"a".repeat(64)).unwrap_err();
    assert_eq!(error.kind(), JsErrorKind::RangeError);
    assert_eq!(
        error.message(),
        "RegExp execution exceeded the configured step limit"
    );
}

#[test]
fn regexp_flag_and_last_index_getters() {
    let re = JsRegExp::new("a", "gim").unwrap();
    assert!(re.global());
    assert!(re.ignore_case());
    assert!(re.multiline());
    assert_eq!(re.last_index(), 0);
    re.set_last_index(3).unwrap();
    assert_eq!(re.last_index(), 3);

    let re = JsRegExp::new("a", "").unwrap();
    assert!(!re.global());
    assert!(!re.ignore_case());
    assert!(!re.multiline());
}

#[test]
fn regexp_exec_advances_and_resets_last_index() {
    let re = JsRegExp::new(r"\d+", "g").unwrap();
    let first = re.exec("a1b22c333").unwrap().unwrap();
    assert_eq!(first.text(), "1");
    assert_eq!(first.index(), 1);
    assert_eq!(first.input(), "a1b22c333");
    assert_eq!(re.last_index(), 2);

    let second = re.exec("a1b22c333").unwrap().unwrap();
    assert_eq!((second.text(), second.index()), ("22".to_string(), 3));
    assert_eq!(re.last_index(), 5);

    let third = re.exec("a1b22c333").unwrap().unwrap();
    assert_eq!((third.text(), third.index()), ("333".to_string(), 6));
    assert_eq!(re.last_index(), 9);

    // Exhausted: null result resets lastIndex to 0, then matching restarts.
    assert!(re.exec("a1b22c333").unwrap().is_none());
    assert_eq!(re.last_index(), 0);
    assert_eq!(re.exec("a1b22c333").unwrap().unwrap().text(), "1");

    // lastIndex beyond the input: no match, reset to 0.
    re.set_last_index(99).unwrap();
    assert!(re.exec("a1").unwrap().is_none());
    assert_eq!(re.last_index(), 0);

    // Negative lastIndex behaves like 0 (ToLength clamp).
    re.set_last_index(-5).unwrap();
    assert_eq!(re.exec("a1").unwrap().unwrap().index(), 1);
}

#[test]
fn regexp_exec_without_g_ignores_state() {
    let re = JsRegExp::new("o", "").unwrap();
    re.set_last_index(2).unwrap();
    let m = re.exec("foo").unwrap().unwrap();
    assert_eq!((m.text(), m.index()), ("o".to_string(), 1));
    // Non-global exec never touches lastIndex.
    assert_eq!(re.last_index(), 2);
    assert_eq!(re.exec("foo").unwrap().unwrap().index(), 1);
}

#[test]
fn regexp_exec_reports_utf16_indexes_and_last_index() {
    let re = JsRegExp::new(r"\d", "g").unwrap();
    let m = re.exec("你1好2").unwrap().unwrap();
    assert_eq!((m.text(), m.index()), ("1".to_string(), 1));
    assert_eq!(re.last_index(), 2);
    let m = re.exec("你1好2").unwrap().unwrap();
    assert_eq!((m.text(), m.index()), ("2".to_string(), 3));
}

#[test]
fn regexp_match_carrier_exposes_groups() {
    let re = JsRegExp::new(r"(\w+)@(\w+)|(!)", "").unwrap();
    let m = re.match_first("mail: a@b").unwrap().unwrap();
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

    assert!(re.match_first("no matches here?").unwrap().is_none());
}

#[test]
fn regexp_match_strings_collects_all_texts_or_none() {
    let re = JsRegExp::new(r"\d+", "g").unwrap();
    assert_eq!(
        dense_values(&re.match_strings("a1b22c333").unwrap().unwrap()),
        vec!["1", "22", "333"]
    );
    assert!(re.match_strings("abc").unwrap().is_none());

    // Empty matches advance without spinning (JS: "baab".match(/a*/g)).
    let re = JsRegExp::new("a*", "g").unwrap();
    assert_eq!(
        dense_values(&re.match_strings("baab").unwrap().unwrap()),
        vec!["", "aa", "", ""]
    );
}

#[test]
fn regexp_global_test_advances_and_resets_last_index() {
    // test delegates to exec: g starts at lastIndex, advances to the match
    // end (UTF-16 code units), and resets to 0 when nothing matches.
    let re = JsRegExp::new(r"\d+", "g").unwrap();
    assert!(re.test("a1b22c333").unwrap());
    assert_eq!(re.last_index(), 2);
    assert!(re.test("a1b22c333").unwrap());
    assert_eq!(re.last_index(), 5);
    assert!(re.test("a1b22c333").unwrap());
    assert_eq!(re.last_index(), 9);
    assert!(!re.test("a1b22c333").unwrap());
    assert_eq!(re.last_index(), 0);
    assert!(re.test("a1b22c333").unwrap());
    assert_eq!(re.last_index(), 2);

    // lastIndex counts UTF-16 code units (你 is one, but 😀 would be two).
    let re = JsRegExp::new(r"\d", "g").unwrap();
    assert!(re.test("你1好2").unwrap());
    assert_eq!(re.last_index(), 2);

    // Non-global test is stateless and ignores lastIndex.
    let re = JsRegExp::new("o", "").unwrap();
    re.set_last_index(2).unwrap();
    assert!(re.test("foo").unwrap());
    assert_eq!(re.last_index(), 2);
    assert!(re.test("foo").unwrap());
}

#[test]
fn regexp_nullable_pattern_over_astral_input_fails_closed() {
    // JS advances one UTF-16 code unit after an empty match, splitting the
    // surrogate pair of an astral char — unrepresentable in a Rust String —
    // so every iterating operation rejects deterministically.
    let split_err = JsRegExp::new("x?", "").unwrap().split("a💚b").unwrap_err();
    assert_eq!(split_err.kind(), JsErrorKind::Unsupported);

    let replace_err = JsRegExp::new("a*", "g")
        .unwrap()
        .replace("a💚b", "-")
        .unwrap_err();
    assert_eq!(replace_err.kind(), JsErrorKind::Unsupported);

    let match_err = JsRegExp::new("(?:a|)", "g")
        .unwrap()
        .match_strings("💚")
        .unwrap_err();
    assert_eq!(match_err.kind(), JsErrorKind::Unsupported);

    let match_all_err = JsRegExp::new("b{0,2}", "g")
        .unwrap()
        .match_all("💚")
        .unwrap_err();
    assert_eq!(match_all_err.kind(), JsErrorKind::Unsupported);

    // Bare anchors are nullable too.
    let anchor_err = JsRegExp::new("^", "gm")
        .unwrap()
        .split("💚\n💚")
        .unwrap_err();
    assert_eq!(anchor_err.kind(), JsErrorKind::Unsupported);

    // Non-g replace never iterates empty matches: always Ok, exact.
    let re = JsRegExp::new("a*", "").unwrap();
    assert_eq!(re.replace("a💚b", "-").unwrap(), "-💚b");
}

#[test]
fn regexp_nullable_pattern_rejects_manual_last_index() {
    // Node: /a*/g with lastIndex = 1 on "💚" matches "" at UTF-16 index 1 —
    // inside the surrogate pair, a position no Rust String can express — so
    // manual lastIndex writes on nullable patterns fail closed at the
    // setter, deterministically and regardless of value or input.
    let re = JsRegExp::new("a*", "g").unwrap();
    let err = re.set_last_index(1).unwrap_err();
    assert_eq!(err.kind(), JsErrorKind::Unsupported);
    assert!(err.message().contains("outside the oracle-proven subset"));
    // The write is rejected before mutating state, and repeats identically.
    assert_eq!(re.last_index(), 0);
    assert_eq!(
        re.set_last_index(0).unwrap_err().kind(),
        JsErrorKind::Unsupported
    );

    // Other nullable shapes reject too, global or not.
    for (pattern, flags) in [("x?", "g"), ("(?:a|)", "g"), ("^", "gm"), ("a*", "")] {
        let nullable = JsRegExp::new(pattern, flags).unwrap();
        assert_eq!(
            nullable.set_last_index(2).unwrap_err().kind(),
            JsErrorKind::Unsupported,
            "pattern {pattern:?} flags {flags:?}"
        );
    }
}

#[test]
fn regexp_nullable_exec_over_astral_input_stays_exact() {
    // Natural-flow exec keeps lastIndex on char boundaries even for
    // nullable patterns over astral input, so it stays exact without the
    // setter. Node: /a*/g on "💚a" matches "" at index 0 on every call
    // (the empty match leaves lastIndex at 0), never entering the pair.
    let re = JsRegExp::new("a*", "g").unwrap();
    for _ in 0..3 {
        let m = re.exec("💚a").unwrap().unwrap();
        assert_eq!((m.text().as_str(), m.index()), ("", 0));
        assert_eq!(re.last_index(), 0);
    }
}

#[test]
fn regexp_non_nullable_patterns_over_astral_input_stay_exact() {
    // Node: "a💚b💚c".replace(/💚/g, "x") === "axbxc"
    let re = JsRegExp::new("💚", "g").unwrap();
    assert_eq!(re.replace("a💚b💚c", "x").unwrap(), "axbxc");

    // Node: "a💚b".split(/💚/) → ["a", "b"]
    assert_eq!(
        dense_values(&JsRegExp::new("💚", "").unwrap().split("a💚b").unwrap()),
        vec!["a", "b"]
    );

    // Node: "💚1💚22".match(/\d+/g) → ["1", "22"]
    assert_eq!(
        dense_values(
            &JsRegExp::new(r"\d+", "g")
                .unwrap()
                .match_strings("💚1💚22")
                .unwrap()
                .unwrap(),
        ),
        vec!["1", "22"]
    );

    // Node: [..."a💚💚b1".matchAll(/💚|\d/g)] — UTF-16 indexes 1, 3, 6.
    // (The astral literal stays unquantified: without the `u` flag Node
    // reads 💚 as two code units, so `💚+` would bind `+` to the trailing
    // surrogate only.)
    let matches = JsRegExp::new(r"💚|\d", "g")
        .unwrap()
        .match_all("a💚💚b1")
        .unwrap();
    assert_eq!(matches.len(), 3);
    assert_eq!((matches[0].text().as_str(), matches[0].index()), ("💚", 1));
    assert_eq!((matches[1].text().as_str(), matches[1].index()), ("💚", 3));
    assert_eq!((matches[2].text().as_str(), matches[2].index()), ("1", 6));
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
    assert_eq!(matches[0].len(), 3);
    assert!(!matches[0].is_empty());
    assert_eq!(matches[1].text(), "b");
    assert_eq!(matches[1].group(2), None);
    assert_eq!(matches[1].len(), 3);
    assert_eq!(matches[1].index(), 3);

    // matchAll is stateless: lastIndex is not consulted or mutated.
    let re = JsRegExp::new("a", "g").unwrap();
    re.set_last_index(2).unwrap();
    assert_eq!(re.match_all("aaa").unwrap().len(), 3);
    assert_eq!(re.last_index(), 2);
}
