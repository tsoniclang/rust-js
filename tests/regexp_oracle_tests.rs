//! Asserts the RegExp subset engine against Node-generated oracle vectors.
//!
//! The vectors are produced by `tools/generate-regexp-oracle.mjs` (run with
//! Node) and committed at `tests/oracle/regexp-vectors.json`. Every construct
//! the engine accepts must behave exactly like Node's RegExp on these
//! vectors.

use std::fs;
use std::path::Path;

use tsonic_rust_js::json;
use tsonic_rust_js::regexp::JsRegExp;
use tsonic_rust_js::value::JsValue;

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

#[test]
fn regexp_engine_matches_node_oracle_vectors() {
    let vectors = load_vectors();
    assert!(
        vectors.len() >= 120,
        "expected at least 120 oracle vectors, found {}",
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

        let regexp = match JsRegExp::new(&pattern, &flags) {
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
            "replace" => {
                let replacement = get_string(entry, "replacement");
                match expected {
                    JsValue::String(expected) => {
                        let actual = regexp.replace(&input, &replacement);
                        (actual == expected).then_some(()).ok_or(format!(
                            "{label} with {replacement:?}: expected {expected:?}, got {actual:?}"
                        ))
                    }
                    other => Err(format!("{label}: bad expected value {other}")),
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
