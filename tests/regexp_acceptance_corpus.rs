//! Engine-generated RegExp acceptance corpus.
//!
//! The corpus at `tests/oracle/regexp-acceptance-corpus.json` is the shared
//! construction-time contract between this engine (`JsRegExp::new`) and the
//! tsonic-rust compile-time validator (`rustRegExpSubsetViolation`). Every
//! entry records whether the engine accepts a `(pattern, flags)` pair and,
//! for rejects, the exact error message.
//!
//! Regeneration: run with `TSONIC_REGEN_CORPUS=1` to rewrite the json from
//! the embedded pattern list below, then rerun normally. The regen path
//! always fails the test so CI can never silently regenerate the contract.

use std::fs;
use std::path::PathBuf;

use tsonic_rust_js::json;
use tsonic_rust_js::regexp::JsRegExp;
use tsonic_rust_js::value::JsValue;

/// Every `(pattern, flags)` pair in the shared acceptance contract. The
/// engine's verdict on each pair — not this list — is what lands in the
/// json: whatever `JsRegExp::new` says goes.
const CORPUS_PATTERNS: &[(&str, &str)] = &[
    // --- reviewer probes -------------------------------------------------
    ("{", ""),
    ("}", ""),
    ("+a", ""),
    ("^*", ""),
    ("a{1001}", ""),
    ("a{2,1}", ""),
    ("a{1,2}?", ""),
    ("[z-a]", ""),
    (r"[\d-x]", ""),
    (r"[a-\n]", ""),
    (r"\e", ""),
    (r"\A", ""),
    (r"\01", ""),
    (r"[\b]", ""),
    // --- code-unit-sensitive constructs ----------------------------------
    (".", ""),
    ("a.b", ""),
    ("[^a]", ""),
    (r"\D", ""),
    (r"\W", ""),
    (r"\S", ""),
    (r"[\D]", ""),
    (r"[\W]", ""),
    (r"[\S]", ""),
    ("[a-\u{e000}]", ""),
    (r"[\x00-￿]", ""),
    (r"[a-\uD800]", ""),
    (r"[\uD800]", ""),
    ("[😀]", ""),
    ("😀?", ""),
    ("😀*", ""),
    ("😀+", ""),
    ("😀{2}", ""),
    (r"\😀?", ""),
    // --- lookaround / groups / backreferences ----------------------------
    ("(?=a)", ""),
    ("(?!a)", ""),
    ("(?<=a)", ""),
    ("(?<!a)", ""),
    ("(?<n>a)", ""),
    ("(?a)", ""),
    (r"(a)\1", ""),
    (r"\8", ""),
    ("(a", ""),
    ("a)", ""),
    ("(", ""),
    (")", ""),
    // --- quantifiers ------------------------------------------------------
    ("a*?", ""),
    ("a+?", ""),
    ("a??", ""),
    ("a**", ""),
    ("^+", ""),
    ("$?", ""),
    ("^{2}", ""),
    ("{2}", ""),
    ("a{}", ""),
    ("a{,2}", ""),
    ("a{2", ""),
    ("a{9999999999}", ""),
    // --- escapes ----------------------------------------------------------
    (r"\b", ""),
    (r"\B", ""),
    (r"a\b", ""),
    (r"\p{L}", ""),
    (r"\P{L}", ""),
    (r"\k<n>", ""),
    (r"\cA", ""),
    (r"[\cA]", ""),
    (r"\u{1F600}", ""),
    (r"\uD800", ""),
    (r"\00", ""),
    (r"[\1]", ""),
    (r"[\B]", ""),
    (r"\x4z", ""),
    (r"\x", ""),
    (r"\u12", ""),
    (r"\uZZZZ", ""),
    ("a\\", ""),
    ("[a", ""),
    (r"[x-\d]", ""),
    (r"[\w-a]", ""),
    // --- flags ------------------------------------------------------------
    ("a", "u"),
    ("a", "y"),
    ("a", "s"),
    ("a", "d"),
    ("a", "v"),
    ("a", "x"),
    ("a", "gg"),
    ("a", "ii"),
    // --- accepted set -----------------------------------------------------
    ("abc", ""),
    ("", ""),
    ("a|b", ""),
    ("foo|bar|baz", ""),
    ("[a-z]+", ""),
    ("[A-Za-z0-9_]*", ""),
    ("[abc]", ""),
    (r"\d", ""),
    (r"\w", ""),
    (r"\s", ""),
    (r"\d+", ""),
    (r"[\d\w\s]", ""),
    (r"\n", ""),
    (r"\t", ""),
    (r"\r", ""),
    (r"\f", ""),
    (r"\v", ""),
    (r"\0", ""),
    (r"\x41", ""),
    (r"A", ""),
    (r"\u0041", ""),
    (r"\.", ""),
    (r"\\", ""),
    (r"\/", ""),
    (r"\+", ""),
    (r"\[", ""),
    (r"\]", ""),
    (r"\{", ""),
    (r"\}", ""),
    (r"\$", ""),
    (r"\^", ""),
    (r"\*", ""),
    (r"\(", ""),
    (r"\)", ""),
    (r"\|", ""),
    (r"\?", ""),
    (r"\-", ""),
    ("(?:a|b)+", ""),
    ("(a)(b)", ""),
    ("(a|b)*c", ""),
    ("^a$", "m"),
    ("^abc$", ""),
    ("a{2}", ""),
    ("a{2,}", ""),
    ("a{2,3}", ""),
    ("a{0,1000}", ""),
    ("a{1000}", ""),
    ("[a-]", ""),
    ("[-a]", ""),
    ("[a-z-]", ""),
    ("[-]", ""),
    (r"[\n-a]", ""),
    (r"[\x00-\x7F]", ""),
    (r"[a-퟿]", ""),
    ("(?:😀)+", ""),
    ("😀", ""),
    ("a", "i"),
    ("a", "g"),
    ("a", "m"),
    ("a", "gim"),
    ("a", "ig"),
    ("a*", ""),
    ("a+", ""),
    ("a?", ""),
    ("(ab)+", ""),
    ("[.]", ""),
    ("[{]", ""),
    ("[}]", ""),
    ("[+*?]", ""),
    ("()", ""),
    ("(|)", ""),
];

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/oracle/regexp-acceptance-corpus.json")
}

fn json_escape(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out
}

fn regenerate_corpus() {
    let mut lines = Vec::new();
    for (pattern, flags) in CORPUS_PATTERNS {
        let entry = match JsRegExp::new(pattern, flags) {
            Ok(_) => format!(
                "  {{\"pattern\": \"{}\", \"flags\": \"{}\", \"accepted\": true}}",
                json_escape(pattern),
                json_escape(flags)
            ),
            Err(error) => format!(
                "  {{\"pattern\": \"{}\", \"flags\": \"{}\", \"accepted\": false, \"reason\": \"{}\"}}",
                json_escape(pattern),
                json_escape(flags),
                json_escape(error.message())
            ),
        };
        lines.push(entry);
    }
    let text = format!("[\n{}\n]\n", lines.join(",\n"));
    fs::write(corpus_path(), text).expect("write regenerated corpus");
}

fn object_field(entry: &JsValue, key: &str) -> JsValue {
    entry
        .as_object()
        .expect("corpus entry object")
        .borrow()
        .get(key)
}

fn get_string(entry: &JsValue, key: &str) -> String {
    match object_field(entry, key) {
        JsValue::String(value) => value,
        other => panic!("expected string for `{key}`, got {other}"),
    }
}

fn get_bool(entry: &JsValue, key: &str) -> bool {
    match object_field(entry, key) {
        JsValue::Bool(value) => value,
        other => panic!("expected boolean for `{key}`, got {other}"),
    }
}

#[test]
fn engine_acceptance_matches_committed_corpus() {
    if std::env::var("TSONIC_REGEN_CORPUS").as_deref() == Ok("1") {
        regenerate_corpus();
        panic!(
            "corpus regenerated at {}; rerun without TSONIC_REGEN_CORPUS to verify",
            corpus_path().display()
        );
    }

    let text = fs::read_to_string(corpus_path()).expect("read acceptance corpus");
    let parsed = json::parse(&text).expect("parse acceptance corpus");
    let array = parsed.as_array().expect("corpus array");
    let entries: Vec<JsValue> = array
        .values()
        .into_iter()
        .map(|value| value.expect("dense corpus entry"))
        .collect();

    assert_eq!(
        entries.len(),
        CORPUS_PATTERNS.len(),
        "corpus entry count drifted from the embedded pattern list; regenerate with TSONIC_REGEN_CORPUS=1"
    );

    for (index, entry) in entries.iter().enumerate() {
        let pattern = get_string(entry, "pattern");
        let flags = get_string(entry, "flags");
        let accepted = get_bool(entry, "accepted");
        let (expected_pattern, expected_flags) = CORPUS_PATTERNS[index];
        assert_eq!(
            (pattern.as_str(), flags.as_str()),
            (expected_pattern, expected_flags),
            "corpus entry {index} drifted from the embedded pattern list; regenerate with TSONIC_REGEN_CORPUS=1"
        );
        match JsRegExp::new(&pattern, &flags) {
            Ok(_) => assert!(
                accepted,
                "engine accepts /{pattern}/{flags} but corpus says rejected"
            ),
            Err(error) => {
                assert!(
                    !accepted,
                    "engine rejects /{pattern}/{flags} (`{}`) but corpus says accepted",
                    error.message()
                );
                let reason = get_string(entry, "reason");
                assert_eq!(
                    reason,
                    error.message(),
                    "rejection reason drifted for /{pattern}/{flags}"
                );
            }
        }
    }
}
