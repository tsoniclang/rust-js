//! Construction-time ECMAScript RegExp grammar acceptance.
//!
//! These cases span the grammar modes and Annex-B boundaries represented by
//! the canonical JavaScript source profile. A valid normative construct is
//! never recorded as an expected rejection.

use tsonic_rust_js::regexp::JsRegExp;
use tsonic_rust_runtime::JsErrorKind;

const VALID_PATTERNS: &[(&str, &str)] = &[
    ("", ""),
    ("abc", ""),
    ("a|b", ""),
    ("(?:a|b)+", ""),
    ("(?:(a)(b))+", ""),
    ("a*?", ""),
    ("a+?", ""),
    ("a??", ""),
    ("a{1,2}?", ""),
    ("(?=a)", ""),
    ("(?!a)", ""),
    ("(?<=a)b", ""),
    ("(?<!a)b", ""),
    ("(?<name>a)", ""),
    (r"(?<name>a)\k<name>", ""),
    (r"(a)\1", ""),
    (r"\1(a)", ""),
    (r"\p{Letter}+", "u"),
    (r"\P{Script=Latin}+", "u"),
    (r"\u{1F600}", "u"),
    (".", "s"),
    (".", "u"),
    (r"[\p{ASCII}&&\p{Letter}]+", "v"),
    (r"[[a-z]--[aeiou]]+", "v"),
    (r"[\q{ab|cd}]", "v"),
    ("(?i:a)", ""),
    ("(?im-s:^a.$)", ""),
    ("(?<x>a)|(?<x>b)", ""),
    ("[^]", ""),
    ("[]", ""),
    (r"\8", ""),
    ("{", ""),
    ("}", ""),
    ("a{", ""),
    (r"\cA", ""),
    (r"[\cA]", ""),
    (r"\0", ""),
    (r"\01", ""),
    ("[a-]", ""),
    ("[-a]", ""),
    ("a", "d"),
    ("a", "g"),
    ("a", "i"),
    ("a", "m"),
    ("a", "s"),
    ("a", "u"),
    ("a", "v"),
    ("a", "y"),
    ("a", "dgimsy"),
    ("a", "dgimvy"),
];

const INVALID_PATTERNS: &[(&str, &str)] = &[
    ("+a", ""),
    ("^*", ""),
    ("a{2,1}", ""),
    ("[z-a]", ""),
    ("[", ""),
    ("(", ""),
    (r"\", ""),
    ("(?<a>a)(?<a>b)", ""),
    ("(?<1>a)", ""),
    ("(?q:a)", ""),
    (r"\p{NoSuchProperty}", "u"),
    (r"\u{110000}", "u"),
    ("[a--b]", "u"),
    (r"[\q{ab}]", "u"),
    ("a", "uv"),
    ("a", "gg"),
    ("a", "q"),
];

#[test]
fn every_normative_grammar_case_is_accepted() {
    let failures = VALID_PATTERNS
        .iter()
        .filter_map(|(pattern, flags)| {
            JsRegExp::new(*pattern, *flags)
                .err()
                .map(|error| format!("/{pattern}/{flags}: {error:?}"))
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "{} valid ECMAScript pattern(s) were rejected:\n - {}",
        failures.len(),
        failures.join("\n - "),
    );
}

#[test]
fn every_invalid_grammar_case_is_a_syntax_error() {
    let failures = INVALID_PATTERNS
        .iter()
        .filter_map(|(pattern, flags)| match JsRegExp::new(*pattern, *flags) {
            Err(error) if error.kind() == JsErrorKind::SyntaxError => None,
            Err(error) => Some(format!(
                "/{pattern}/{flags}: expected SyntaxError, got {error:?}",
            )),
            Ok(_) => Some(format!("/{pattern}/{flags}: unexpectedly accepted")),
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "{} invalid ECMAScript pattern(s) had the wrong result:\n - {}",
        failures.len(),
        failures.join("\n - "),
    );
}
