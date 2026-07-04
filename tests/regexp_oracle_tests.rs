//! Asserts the RegExp subset engine against Node-generated oracle vectors.
//!
//! The vectors are produced by `tools/generate-regexp-oracle.mjs` (run with
//! Node) and committed at `tests/oracle/regexp-vectors.json`. Every construct
//! the engine accepts must behave exactly like Node's RegExp on these
//! vectors.

use std::fs;
use std::path::Path;

use tsonic_rust_js::json;
use tsonic_rust_js::regexp::{JsRegExp, JsRegExpMatch};
use tsonic_rust_js::value::JsValue;
use tsonic_rust_js::JsErrorKind;

fn load_vectors() -> Vec<JsValue> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/oracle/regexp-vectors.json")
        .canonicalize()
        .expect("oracle vector path");
    let text = fs::read_to_string(&path).expect("read oracle vectors");
    let parsed = json::parse(&text).expect("parse oracle vectors");
    let array = parsed.as_array().expect("vector array").borrow().clone();
    array
        .values()
        .into_iter()
        .map(|value| value.expect("dense vector entry").clone())
        .collect()
}

fn get_string(entry: &JsValue, key: &str) -> String {
    match object_field(entry, key) {
        JsValue::String(value) => value,
        other => panic!("expected string for `{key}`, got {other}"),
    }
}

fn object_field(entry: &JsValue, key: &str) -> JsValue {
    entry
        .as_object()
        .expect("vector entry object")
        .borrow()
        .get(key)
}

fn get_number(entry: &JsValue, key: &str) -> f64 {
    match object_field(entry, key) {
        JsValue::Number(value) => value,
        other => panic!("expected number for `{key}`, got {other}"),
    }
}

fn array_items(value: &JsValue) -> Vec<JsValue> {
    value
        .as_array()
        .expect("expected array value")
        .borrow()
        .values()
        .into_iter()
        .map(|item| item.expect("dense array item").clone())
        .collect()
}

/// Compares a `JsRegExpMatch` against an oracle `{text, index, groups}`
/// record (groups are `null` for unmatched optional groups).
fn check_match_record(
    label: &str,
    expected: &JsValue,
    actual: &JsRegExpMatch,
    input: &str,
) -> Result<(), String> {
    let text = get_string(expected, "text");
    let index = get_number(expected, "index");
    if actual.text() != text || f64::from(actual.index()) != index {
        return Err(format!(
            "{label}: expected match {text:?} at {index}, got {:?} at {}",
            actual.text(),
            actual.index()
        ));
    }
    if actual.input() != input {
        return Err(format!("{label}: match input diverged"));
    }
    let groups = array_items(&object_field(expected, "groups"));
    if actual.group_count() != groups.len() {
        return Err(format!(
            "{label}: expected {} groups, got {}",
            groups.len(),
            actual.group_count()
        ));
    }
    for (offset, group) in groups.iter().enumerate() {
        let expected_group = match group {
            JsValue::Null => None,
            JsValue::String(value) => Some(value.clone()),
            other => return Err(format!("{label}: bad group value {other}")),
        };
        if actual.group(offset + 1) != expected_group {
            return Err(format!(
                "{label}: group {} expected {expected_group:?}, got {:?}",
                offset + 1,
                actual.group(offset + 1)
            ));
        }
    }
    Ok(())
}

/// Drives `calls` sequential `test` calls on one regexp instance, asserting
/// each boolean result and the `lastIndex` progression/reset recorded by
/// Node (a global `test` delegates to `exec`; a non-global one is
/// stateless).
fn check_test_steps(
    label: &str,
    steps: &[JsValue],
    regexp: &mut JsRegExp,
    input: &str,
) -> Result<(), String> {
    for (call, step) in steps.iter().enumerate() {
        let step_label = format!("{label} call {call}");
        let actual = regexp.test(input);
        let expected = match object_field(step, "result") {
            JsValue::Bool(value) => value,
            other => return Err(format!("{step_label}: bad expected result {other}")),
        };
        if actual != expected {
            return Err(format!("{step_label}: expected {expected}, got {actual}"));
        }
        let expected_last = get_number(step, "lastIndex");
        if f64::from(regexp.last_index()) != expected_last {
            return Err(format!(
                "{step_label}: expected lastIndex {expected_last}, got {}",
                regexp.last_index()
            ));
        }
    }
    Ok(())
}

/// Drives `calls` sequential `exec` calls on one regexp instance, asserting
/// each result and the `lastIndex` progression/reset recorded by Node.
fn check_exec_steps(
    label: &str,
    steps: &[JsValue],
    regexp: &mut JsRegExp,
    input: &str,
) -> Result<(), String> {
    for (call, step) in steps.iter().enumerate() {
        let step_label = format!("{label} call {call}");
        let actual = regexp.exec(input);
        match (object_field(step, "match"), actual) {
            (JsValue::Null, None) => {}
            (JsValue::Null, Some(actual)) => {
                return Err(format!(
                    "{step_label}: expected null, got {:?}",
                    actual.text()
                ));
            }
            (expected @ JsValue::Object(_), Some(actual)) => {
                check_match_record(&step_label, &expected, &actual, input)?;
            }
            (expected, None) => {
                return Err(format!("{step_label}: expected {expected}, got null"));
            }
            (other, _) => return Err(format!("{step_label}: bad expected value {other}")),
        }
        let expected_last = get_number(step, "lastIndex");
        if f64::from(regexp.last_index()) != expected_last {
            return Err(format!(
                "{step_label}: expected lastIndex {expected_last}, got {}",
                regexp.last_index()
            ));
        }
    }
    Ok(())
}

#[test]
fn regexp_engine_matches_node_oracle_vectors() {
    let vectors = load_vectors();
    assert!(
        vectors.len() >= 216,
        "expected at least 216 oracle vectors, found {}",
        vectors.len()
    );

    let mut failures = Vec::new();
    for entry in &vectors {
        let pattern = get_string(entry, "pattern");
        let flags = get_string(entry, "flags");
        let input = get_string(entry, "input");
        let op = get_string(entry, "op");
        let expected = object_field(entry, "expected");
        let label = format!("/{pattern}/{flags} `{op}` on {input:?}");

        let mut regexp = match JsRegExp::new(&pattern, &flags) {
            Ok(regexp) => regexp,
            Err(error) => {
                failures.push(format!("{label}: engine rejected pattern: {error:?}"));
                continue;
            }
        };

        let outcome = match op.as_str() {
            "test" => match expected {
                JsValue::Bool(expected) => {
                    let actual = regexp.test(&input);
                    (actual == expected)
                        .then_some(())
                        .ok_or(format!("{label}: expected {expected}, got {actual}"))
                }
                other => Err(format!("{label}: bad expected value {other}")),
            },
            "search" => match expected {
                JsValue::Number(expected) => {
                    let actual = regexp.search(&input);
                    (f64::from(actual) == expected)
                        .then_some(())
                        .ok_or(format!("{label}: expected {expected}, got {actual}"))
                }
                other => Err(format!("{label}: bad expected value {other}")),
            },
            "test-sequence" => {
                check_test_steps(&label, &array_items(&expected), &mut regexp, &input)
            }
            "replace" => {
                let replacement = get_string(entry, "replacement");
                match (expected, regexp.replace(&input, &replacement)) {
                    (JsValue::String(expected), Ok(actual)) => {
                        (actual == expected).then_some(()).ok_or(format!(
                            "{label} with {replacement:?}: expected {expected:?}, got {actual:?}"
                        ))
                    }
                    (_, Err(error)) => Err(format!("{label}: replace rejected: {error:?}")),
                    (other, _) => Err(format!("{label}: bad expected value {other}")),
                }
            }
            "split" => {
                let expected = match expected.as_array() {
                    Some(values) => values
                        .borrow()
                        .values()
                        .into_iter()
                        .map(|value| match value {
                            Some(JsValue::String(part)) => part.clone(),
                            other => panic!("{label}: bad split part {other:?}"),
                        })
                        .collect::<Vec<_>>(),
                    None => panic!("{label}: expected array"),
                };
                match regexp.split(&input) {
                    Ok(actual) => (actual == expected)
                        .then_some(())
                        .ok_or(format!("{label}: expected {expected:?}, got {actual:?}")),
                    Err(error) => Err(format!("{label}: split rejected: {error:?}")),
                }
            }
            "exec" => check_exec_steps(&label, &array_items(&expected), &mut regexp, &input),
            // Writes `setLastIndex` (a UTF-16 offset that may land inside a
            // surrogate pair) via `set_last_index`, runs one `exec`, and
            // asserts the match text and resulting `lastIndex` recorded from
            // Node — proving mid-pair starts are equivalent to the next char
            // boundary for the accepted subset.
            "set-lastindex" => {
                regexp.set_last_index(get_number(entry, "setLastIndex") as i32);
                let actual = regexp.exec(&input);
                let text_outcome = match (object_field(&expected, "result"), &actual) {
                    (JsValue::Null, None) => Ok(()),
                    (JsValue::String(text), Some(matched)) if matched.text() == text => Ok(()),
                    (expected_result, _) => Err(format!(
                        "{label} from {}: expected {expected_result}, got {:?}",
                        get_number(entry, "setLastIndex"),
                        actual.as_ref().map(JsRegExpMatch::text)
                    )),
                };
                text_outcome.and_then(|()| {
                    let expected_last = get_number(&expected, "lastIndex");
                    (f64::from(regexp.last_index()) == expected_last)
                        .then_some(())
                        .ok_or(format!(
                            "{label}: expected lastIndex {expected_last}, got {}",
                            regexp.last_index()
                        ))
                })
            }
            "match" => {
                if flags.contains('g') {
                    let expected = match &expected {
                        JsValue::Null => None,
                        other => Some(
                            array_items(other)
                                .iter()
                                .map(|item| match item {
                                    JsValue::String(text) => text.clone(),
                                    other => panic!("{label}: bad match text {other}"),
                                })
                                .collect::<Vec<_>>(),
                        ),
                    };
                    match regexp.match_strings(&input) {
                        Ok(actual) => (actual == expected)
                            .then_some(())
                            .ok_or(format!("{label}: expected {expected:?}, got {actual:?}")),
                        Err(error) => Err(format!("{label}: match rejected: {error:?}")),
                    }
                } else {
                    match (&expected, regexp.match_first(&input)) {
                        (JsValue::Null, None) => Ok(()),
                        (JsValue::Null, Some(actual)) => {
                            Err(format!("{label}: expected null, got {:?}", actual.text()))
                        }
                        (JsValue::Object(_), Some(actual)) => {
                            check_match_record(&label, &expected, &actual, &input)
                        }
                        (expected, None) => Err(format!("{label}: expected {expected}, got null")),
                        (other, _) => Err(format!("{label}: bad expected value {other}")),
                    }
                }
            }
            "matchAll" => match &expected {
                JsValue::Object(_) => {
                    let throws = get_string(&expected, "throws");
                    assert_eq!(throws, "TypeError", "{label}: unexpected oracle error");
                    match regexp.match_all(&input) {
                        Err(error) if error.kind() == JsErrorKind::TypeError => Ok(()),
                        Err(error) => Err(format!("{label}: expected TypeError, got {error:?}")),
                        Ok(_) => Err(format!("{label}: expected TypeError, got matches")),
                    }
                }
                _ => {
                    let records = array_items(&expected);
                    match regexp.match_all(&input) {
                        Err(error) => Err(format!("{label}: matchAll rejected: {error:?}")),
                        Ok(actual) if actual.len() != records.len() => Err(format!(
                            "{label}: expected {} matches, got {}",
                            records.len(),
                            actual.len()
                        )),
                        Ok(actual) => {
                            records
                                .iter()
                                .zip(&actual)
                                .try_for_each(|(record, matched)| {
                                    check_match_record(&label, record, matched, &input)
                                })
                        }
                    }
                }
            },
            other => Err(format!("{label}: unknown op `{other}`")),
        };
        if let Err(failure) = outcome {
            failures.push(failure);
        }
    }

    assert!(
        failures.is_empty(),
        "{} oracle vector(s) diverged:\n - {}",
        failures.len(),
        failures.join("\n - ")
    );
}
