use std::cell::RefCell;
use std::rc::Rc;

use tsonic_rust_js::regexp::{regexp_escape_exact_native, regexp_exec_native, JsRegExp};
use tsonic_rust_js::{JsArray, JsString, JsValue};
use tsonic_rust_runtime::{JsError, JsErrorKind, TsonicError, Undefined};

fn js(value: &str) -> JsString {
    JsString::from_utf8(value)
}

fn text(value: &JsString) -> String {
    value
        .to_utf8()
        .expect("test text must be well-formed UTF-16")
}

fn dense_strings(values: &JsArray<JsString>) -> Vec<String> {
    values
        .values()
        .into_iter()
        .map(|value| value.map_or_else(|| "<undefined>".to_string(), |value| text(&value)))
        .collect()
}

#[test]
fn regexp_construction_identity_flags_and_source_are_exact() {
    let expression = JsRegExp::new(js("/"), js("ymisgd")).unwrap();
    assert_eq!(text(&expression.source()), "\\/");
    assert_eq!(text(&expression.flags()), "dgimsy");
    assert!(expression.has_indices());
    assert!(expression.global());
    assert!(expression.ignore_case());
    assert!(expression.multiline());
    assert!(expression.dot_all());
    assert!(expression.sticky());
    assert!(!expression.unicode());
    assert!(!expression.unicode_sets());
    assert_eq!(text(&expression.to_string_value()), "/\\//dgimsy");
    assert_eq!(text(&JsRegExp::empty().unwrap().source()), "(?:)");

    let alias = JsRegExp::call_from_regexp(&expression).unwrap();
    let clone = JsRegExp::construct_from_regexp(&expression).unwrap();
    assert_eq!(expression, alias);
    assert_ne!(expression, clone);
    expression.set_last_index(7.0);
    assert_eq!(alias.last_index(), 7.0);
    assert_eq!(clone.last_index(), 0.0);
}

#[test]
fn regexp_constructor_entry_points_preserve_pattern_flags_and_identity() {
    let pattern = js("a+");
    let global = js("g");
    let undefined = Undefined;

    let from_string = JsRegExp::from_string_with_flags(&pattern, &global).unwrap();
    assert_eq!(from_string.pattern(), pattern);
    assert_eq!(from_string.flags(), global);
    assert_eq!(
        JsRegExp::from_string_with_undefined_flags(&pattern, undefined)
            .unwrap()
            .flags(),
        js(""),
    );

    assert_eq!(
        JsRegExp::from_undefined(undefined).unwrap().pattern(),
        js("")
    );
    assert_eq!(
        JsRegExp::from_undefined_with_flags(undefined, &global)
            .unwrap()
            .flags(),
        global,
    );
    assert_eq!(
        JsRegExp::from_undefined_with_undefined_flags(undefined, undefined)
            .unwrap()
            .pattern(),
        js(""),
    );

    from_string.set_last_index(4.0);
    let called_with_undefined =
        JsRegExp::call_from_regexp_with_undefined_flags(&from_string, undefined).unwrap();
    assert_eq!(called_with_undefined, from_string);
    assert_eq!(called_with_undefined.last_index(), 4.0);

    let called_with_flags = JsRegExp::call_from_regexp_with_flags(&from_string, &js("i")).unwrap();
    assert_ne!(called_with_flags, from_string);
    assert_eq!(called_with_flags.pattern(), pattern);
    assert_eq!(called_with_flags.flags(), js("i"));
    assert_eq!(called_with_flags.last_index(), 0.0);

    let constructed_with_flags =
        JsRegExp::construct_from_regexp_with_flags(&from_string, &js("m")).unwrap();
    assert_ne!(constructed_with_flags, from_string);
    assert_eq!(constructed_with_flags.pattern(), pattern);
    assert_eq!(constructed_with_flags.flags(), js("m"));

    let constructed_with_undefined =
        JsRegExp::construct_from_regexp_with_undefined_flags(&from_string, undefined).unwrap();
    assert_ne!(constructed_with_undefined, from_string);
    assert_eq!(constructed_with_undefined.pattern(), pattern);
    assert_eq!(constructed_with_undefined.flags(), global);
    assert_eq!(constructed_with_undefined.last_index(), 0.0);
}

#[test]
fn regexp_result_required_groups_and_replace_all_entry_points_are_exact() {
    let single = JsRegExp::new(js("(a)?b"), js("")).unwrap();
    let execution = single.exec(&js("ab")).unwrap().unwrap();
    assert_eq!(execution.required_group(1.0), js("a"));
    assert_eq!(execution.required_group(2.0), js(""));

    let matched = single.match_result(&js("ab")).unwrap().unwrap();
    assert_eq!(matched.required_group(1.0), js("a"));
    assert_eq!(matched.required_group(2.0), js(""));

    let global = JsRegExp::new(js("a"), js("g")).unwrap();
    assert_eq!(
        global.replace_all_for_string(&js("aba"), &js("x")).unwrap(),
        js("xbx"),
    );
    assert_eq!(
        global
            .replace_all_for_string_with(&js("aba"), |_| js("y"))
            .unwrap(),
        js("yby"),
    );
    assert_eq!(
        global
            .try_replace_all_for_string_with(&js("aba"), |_| Ok::<_, JsError>(js("z")))
            .unwrap(),
        js("zbz"),
    );
}

#[test]
fn regexp_fallible_replacement_preserves_the_callers_error_domain() {
    let expression = JsRegExp::new(js("a"), js("g")).unwrap();
    let callback_error = TsonicError::unsupported("callback failed");
    let result = expression.try_replace_all_for_string_with(&js("aba"), |_| {
        Err::<JsString, _>(callback_error.clone())
    });
    assert_eq!(result.unwrap_err(), callback_error);

    let invalid_operation = JsRegExp::new(js("a"), js("")).unwrap();
    assert!(matches!(
        invalid_operation.try_replace_all_for_string_with(
            &js("aba"),
            |_| Ok::<_, TsonicError>(js("unused")),
        ),
        Err(TsonicError::Js(error)) if error.kind() == JsErrorKind::TypeError
    ));
}

#[test]
fn regexp_modern_grammar_executes_without_subset_fallbacks() {
    let cases = [
        ("a+?", "", "aaaa", "a"),
        (r"(a)\1", "", "zaaz", "aa"),
        (r"a(?=b)", "", "zab", "a"),
        (r"(?<=a)b", "", "zab", "b"),
        (r"(?<word>[a-z]+)", "", "42alpha", "alpha"),
        (r"\p{Script=Greek}+", "u", "aαβz", "αβ"),
        (r"(?i:a)b", "", "Ab", "Ab"),
        (r"[\p{ASCII}&&\p{Letter}]+", "v", "éAb9", "Ab"),
    ];

    for (pattern, flags, input, expected) in cases {
        let result = JsRegExp::new(js(pattern), js(flags))
            .unwrap_or_else(|error| panic!("/{pattern}/{flags} failed to compile: {error:?}"))
            .exec(&js(input))
            .unwrap()
            .unwrap_or_else(|| panic!("/{pattern}/{flags} did not match {input:?}"));
        assert_eq!(text(&result.text()), expected, "/{pattern}/{flags}");
    }
}

#[test]
fn regexp_utf16_mode_and_indices_follow_ecmascript_code_units() {
    let astral = js("😀");
    let legacy = JsRegExp::new(js("."), js("d"))
        .unwrap()
        .exec(&astral)
        .unwrap()
        .unwrap();
    assert_eq!(legacy.text().units(), &[0xD83D]);
    assert_eq!(legacy.index(), 0.0);
    assert_eq!(legacy.indices().unwrap().at(0), Some((0.0, 1.0)));

    let unicode = JsRegExp::new(js("."), js("du"))
        .unwrap()
        .exec(&astral)
        .unwrap()
        .unwrap();
    assert_eq!(unicode.text(), astral);
    assert_eq!(unicode.indices().unwrap().at(0), Some((0.0, 2.0)));

    let sticky = JsRegExp::new(js("b"), js("y")).unwrap();
    sticky.set_last_index(2.0);
    assert_eq!(text(&sticky.exec(&js("😀b")).unwrap().unwrap().text()), "b");
    assert_eq!(sticky.last_index(), 3.0);

    let quantified_astral = JsRegExp::new(js("💚+"), js("")).unwrap();
    let quantified_match = quantified_astral.exec(&js("a💚💚b")).unwrap().unwrap();
    assert_eq!(quantified_match.text(), js("💚"));
    assert_eq!(quantified_match.index(), 1.0);

    let nullable = JsRegExp::new(js("a*"), js("g")).unwrap();
    nullable.set_last_index(1.0);
    let nullable_mid_pair = nullable.exec(&astral).unwrap().unwrap();
    assert_eq!(nullable_mid_pair.text(), js(""));
    assert_eq!(nullable_mid_pair.index(), 1.0);
    assert_eq!(nullable.last_index(), 1.0);

    let nullable_before_scalar = JsRegExp::new(js("a*"), js("g")).unwrap();
    nullable_before_scalar.set_last_index(2.0);
    let nullable_scalar = nullable_before_scalar.exec(&js("😀a")).unwrap().unwrap();
    assert_eq!(nullable_scalar.text(), js("a"));
    assert_eq!(nullable_scalar.index(), 2.0);
    assert_eq!(nullable_before_scalar.last_index(), 3.0);
}

#[test]
fn regexp_named_groups_optional_captures_and_indices_are_preserved() {
    let result = JsRegExp::new(js(r"(?<letter>[a-z]+)(?<digits>\d+)?"), js("d"))
        .unwrap()
        .exec(&js("abc"))
        .unwrap()
        .unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(text(&result.group(1).unwrap()), "abc");
    assert_eq!(result.group(2), None);
    let groups = result.groups().unwrap();
    assert_eq!(text(&groups.get(&js("letter")).unwrap()), "abc");
    assert_eq!(groups.get(&js("digits")), None);
    assert!(groups.has(&js("digits")));

    let indices = result.indices().unwrap();
    assert_eq!(indices.at(0), Some((0.0, 3.0)));
    assert_eq!(indices.at(1), Some((0.0, 3.0)));
    assert_eq!(indices.at(2), None);
    let named = indices.groups().unwrap();
    assert_eq!(named.get(&js("letter")), Some((0.0, 3.0)));
    assert_eq!(named.get(&js("digits")), None);
    assert!(named.has(&js("digits")));
}

#[test]
fn regexp_global_match_and_lazy_match_all_have_independent_state() {
    let input = js("a1b22c333");
    let expression = JsRegExp::new(js(r"\d+"), js("g")).unwrap();
    expression.set_last_index(2.0);
    let matched = expression.match_result(&input).unwrap().unwrap();
    assert_eq!(dense_strings(&matched.array()), ["1", "22", "333"]);
    assert_eq!(expression.last_index(), 0.0);

    expression.set_last_index(3.0);
    let mut iterator = expression.match_all_for_string(&input).unwrap();
    assert_eq!(expression.last_index(), 3.0);
    let first = iterator.next().unwrap().unwrap();
    assert_eq!(text(&first.text()), "22");
    assert_eq!(expression.last_index(), 3.0);
    let remainder = iterator
        .map(|result| text(&result.unwrap().text()))
        .collect::<Vec<_>>();
    assert_eq!(remainder, ["333"]);

    let nonglobal = JsRegExp::new(js(r"\d+"), js("")).unwrap();
    assert_eq!(
        nonglobal.match_all_for_string(&input).unwrap_err().kind(),
        JsErrorKind::TypeError,
    );
    let direct = nonglobal.match_all(&input).unwrap().collect::<Vec<_>>();
    assert_eq!(direct.len(), 1);
}

#[test]
fn regexp_replacement_tokens_and_callback_arguments_are_exact() {
    let input = js("a1b2x");
    let expression = JsRegExp::new(js(r"(?<digit>\d)(x)?"), js("g")).unwrap();
    assert_eq!(
        text(&expression.replace(&input, &js("<$<digit>>")).unwrap()),
        "a<1>b<2>",
    );

    let observed = Rc::new(RefCell::new(Vec::<Vec<JsValue>>::new()));
    let callback_observed = Rc::clone(&observed);
    let output = expression
        .replace_with(&input, move |arguments| {
            callback_observed.borrow_mut().push(
                arguments
                    .values()
                    .into_iter()
                    .map(|value| value.unwrap())
                    .collect(),
            );
            js("#")
        })
        .unwrap();
    assert_eq!(text(&output), "a#b#");

    let calls = observed.borrow();
    assert_eq!(calls.len(), 2);
    assert!(matches!(&calls[0][0], JsValue::String(value) if value == &js("1")));
    assert!(matches!(&calls[0][1], JsValue::String(value) if value == &js("1")));
    assert!(matches!(calls[0][2], JsValue::Undefined));
    assert!(matches!(calls[0][3], JsValue::Number(value) if value == 1.0));
    assert!(matches!(&calls[0][4], JsValue::String(value) if value == &js("a1b2x")));
    assert!(matches!(calls[0][5], JsValue::Object(_)));
    assert!(matches!(&calls[1][2], JsValue::String(value) if value == &js("x")));
}

#[test]
fn regexp_split_includes_captures_limits_and_empty_boundaries() {
    let expression = JsRegExp::new(js(r"(\d+)"), js("")).unwrap();
    assert_eq!(
        dense_strings(&expression.split_all(&js("a1b22c")).unwrap()),
        ["a", "1", "b", "22", "c"],
    );
    assert_eq!(
        dense_strings(&expression.split_with_limit(&js("a1b22c"), 3.0).unwrap()),
        ["a", "1", "b"],
    );
    assert_eq!(
        dense_strings(
            &JsRegExp::new(js(","), js(""))
                .unwrap()
                .split_all(&js(",a,"))
                .unwrap()
        ),
        ["", "a", ""],
    );
    assert!(JsRegExp::new(js(""), js(""))
        .unwrap()
        .split_all(&js(""))
        .unwrap()
        .is_empty());
}

#[test]
fn regexp_search_preserves_observable_last_index() {
    let expression = JsRegExp::new(js("b"), js("gy")).unwrap();
    expression.set_last_index(7.0);
    assert_eq!(expression.search(&js("ab")).unwrap(), -1.0);
    assert_eq!(expression.last_index(), 7.0);

    let expression = JsRegExp::new(js("b"), js("g")).unwrap();
    expression.set_last_index(7.0);
    assert_eq!(expression.search(&js("ab")).unwrap(), 1.0);
    assert_eq!(expression.last_index(), 7.0);
}

#[test]
fn regexp_escape_matches_the_normative_escape_shape() {
    assert_eq!(text(&JsRegExp::escape(&js("foo-bar"))), r"\x66oo\x2dbar");
    assert_eq!(text(&JsRegExp::escape(&js("a+b/c"))), r"\x61\+b\/c");
    assert_eq!(
        JsRegExp::escape(&JsString::from_units(vec![0xD800])).units(),
        r"\ud800".encode_utf16().collect::<Vec<_>>(),
    );
    assert_eq!(
        regexp_escape_exact_native(&JsString::from_units(vec![0xD800])),
        r"\ud800",
    );
}

#[test]
fn native_results_fail_closed_when_only_the_exact_lane_can_represent_them() {
    let exact_expression = JsRegExp::new(js("."), js("")).unwrap();
    let exact_result = exact_expression.exec(&js("😀")).unwrap().unwrap();
    assert_eq!(exact_result.text().units(), &[0xD83D]);

    let native_expression = JsRegExp::new(js("."), js("")).unwrap();
    assert_eq!(
        regexp_exec_native(&native_expression, "😀")
            .unwrap_err()
            .kind(),
        JsErrorKind::TypeError,
    );
}

#[test]
fn regexp_invalid_inputs_and_resource_limits_fail_precisely() {
    for flags in ["gg", "uv", "q"] {
        assert_eq!(
            JsRegExp::new(js("a"), js(flags)).unwrap_err().kind(),
            JsErrorKind::SyntaxError,
            "flags {flags:?}",
        );
    }
    for pattern in ["[", "(", r"\", "a{2,1}"] {
        assert_eq!(
            JsRegExp::new(js(pattern), js("u")).unwrap_err().kind(),
            JsErrorKind::SyntaxError,
            "pattern {pattern:?}",
        );
    }
    let oversized = JsString::from_units(vec![b'a' as u16; 1_048_577]);
    assert_eq!(
        JsRegExp::new(oversized, JsString::new())
            .unwrap_err()
            .kind(),
        JsErrorKind::RangeError,
    );
}

#[test]
fn regexp_compiled_program_reuse_never_shares_mutable_state() {
    let first = JsRegExp::new(js("same"), js("g")).unwrap();
    let second = JsRegExp::new(js("same"), js("g")).unwrap();
    first.set_last_index(3.0);
    assert_eq!(second.last_index(), 0.0);
    assert_ne!(first, second);

    for index in 0..300 {
        let pattern = format!("cache{index}");
        assert!(JsRegExp::new(js(&pattern), js("")).is_ok());
    }
    assert!(JsRegExp::new(js("same"), js("g"))
        .unwrap()
        .test(&js("same"))
        .unwrap());
}
