//! Differential RegExp execution against committed Node oracle vectors.

use std::path::Path;
use std::process::Command;
use num_traits::ToPrimitive;

use tsonic_rust_js::json;
use tsonic_rust_js::regexp::{JsRegExp, JsRegExpExecArray, JsRegExpMatchArray};
use tsonic_rust_js::{JsArray, JsObject, JsString, JsValue};
use tsonic_rust_runtime::JsErrorKind;

fn object_field(entry: &JsValue, key: &str) -> JsValue {
    entry
        .as_object()
        .expect("oracle entry object")
        .borrow()
        .get(key)
}

fn string_field(entry: &JsValue, key: &str) -> JsString {
    match object_field(entry, key) {
        JsValue::Utf16String(value) => value,
        other => panic!("expected string for `{key}`, got {other:?}"),
    }
}

fn number_field(entry: &JsValue, key: &str) -> f64 {
    match object_field(entry, key) {
        JsValue::Number(value) => value,
        other => panic!("expected number for `{key}`, got {other:?}"),
    }
}

fn array_items(value: &JsValue) -> Vec<JsValue> {
    value.as_array().expect("oracle array").values()
}

fn load_vectors() -> Vec<JsValue> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/oracle/regexp-vectors.json");
    let output = Command::new("node")
        .args([
            "--input-type=module",
            "-e",
            r#"
import { readFileSync } from 'node:fs';
const vectors = JSON.parse(readFileSync(process.argv[1], 'utf8'));
process.stdout.write(JSON.stringify(vectors, (_, value) => typeof value === 'string'
  ? { __oracle_utf16: Array.from({ length: value.length }, (_, index) => value.charCodeAt(index)) }
  : value));
"#,
        ])
        .arg(path)
        .output()
        .expect("encode exact UTF-16 oracle strings");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed =
        json::parse(std::str::from_utf8(&output.stdout).unwrap()).expect("parse oracle transport");
    array_items(&restore_exact_strings(parsed))
}

fn restore_exact_strings(value: JsValue) -> JsValue {
    match value {
        JsValue::Array(values) => JsValue::array(JsArray::from_dense(
            array_items(&JsValue::Array(values))
                .into_iter()
                .map(restore_exact_strings)
                .collect(),
        )),
        JsValue::Object(object) => {
            let object = object.borrow();
            if let Some(units) = object.get("__oracle_utf16").as_array() {
                assert_eq!(object.keys().unwrap(), ["__oracle_utf16"]);
                let units: Vec<u16> = array_items(&JsValue::Array(units.clone()))
                    .into_iter()
                    .map(|value| {
                        let JsValue::Number(unit) = value else {
                            panic!("oracle unit must be numeric");
                        };
                        assert!(unit >= 0.0 && unit <= 65535.0 && unit.fract() == 0.0);
                        unit as u16
                    })
                    .collect();
                JsValue::Utf16String(JsString::from_units(units))
            } else {
                let mut restored = JsObject::new();
                for (key, value) in object.entries().unwrap() {
                    restored.set(&key, restore_exact_strings(value));
                }
                JsValue::object(restored)
            }
        }
        value => value,
    }
}

#[derive(Debug)]
struct ExpectedMatch {
    text: JsString,
    index: usize,
    groups: Vec<Option<JsString>>,
}

fn expected_match(value: &JsValue) -> ExpectedMatch {
    let groups = array_items(&object_field(value, "groups"))
        .into_iter()
        .map(|group| match group {
            JsValue::Null => None,
            JsValue::Utf16String(value) => Some(value),
            other => panic!("invalid oracle capture {other:?}"),
        })
        .collect();
    let index = number_field(value, "index");
    assert_eq!(index.fract(), 0.0, "oracle index must be integral");
    ExpectedMatch {
        text: string_field(value, "text"),
        index: index.to_usize().expect("oracle index must fit usize"),
        groups,
    }
}

fn compare_exec_match(
    label: &str,
    expected: &ExpectedMatch,
    actual: &JsRegExpExecArray,
) -> Result<(), String> {
    if actual.text() != expected.text || actual.index() != expected.index {
        return Err(format!(
            "{label}: expected {:?} at {}, got {:?} at {}",
            expected.text,
            expected.index,
            actual.text(),
            actual.index(),
        ));
    }
    compare_groups(label, &expected.groups, actual.group_count(), |index| {
        actual.group(index)
    })
}

fn compare_match_result(
    label: &str,
    expected: &ExpectedMatch,
    actual: &JsRegExpMatchArray,
) -> Result<(), String> {
    if actual.text() != expected.text || actual.index() != Some(expected.index) {
        return Err(format!(
            "{label}: expected {:?} at {}, got {:?} at {:?}",
            expected.text,
            expected.index,
            actual.text(),
            actual.index(),
        ));
    }
    compare_groups(label, &expected.groups, actual.group_count(), |index| {
        actual.group(index)
    })
}

fn compare_groups(
    label: &str,
    expected: &[Option<JsString>],
    actual_count: usize,
    actual: impl Fn(usize) -> Option<JsString>,
) -> Result<(), String> {
    if actual_count != expected.len() {
        return Err(format!(
            "{label}: expected {} captures, got {actual_count}",
            expected.len(),
        ));
    }
    for (offset, expected) in expected.iter().enumerate() {
        let actual = actual(offset + 1);
        if actual != *expected {
            return Err(format!(
                "{label}: capture {} expected {expected:?}, got {actual:?}",
                offset + 1,
            ));
        }
    }
    Ok(())
}

fn compare_exec_sequence(
    label: &str,
    expression: &JsRegExp,
    input: &JsString,
    expected: &[JsValue],
) -> Result<(), String> {
    for (call, step) in expected.iter().enumerate() {
        let call_label = format!("{label}, exec call {call}");
        let actual = expression
            .exec(input)
            .map_err(|error| format!("{call_label}: {error:?}"))?;
        match (object_field(step, "match"), actual) {
            (JsValue::Null, None) => {}
            (JsValue::Object(_), Some(actual)) => {
                compare_exec_match(
                    &call_label,
                    &expected_match(&object_field(step, "match")),
                    &actual,
                )?;
            }
            (expected, actual) => {
                return Err(format!(
                    "{call_label}: expected {expected:?}, got {actual:?}",
                ));
            }
        }
        let expected_last_index = number_field(step, "lastIndex");
        if expression.last_index() != expected_last_index {
            return Err(format!(
                "{call_label}: expected lastIndex {expected_last_index}, got {}",
                expression.last_index(),
            ));
        }
    }
    Ok(())
}

fn compare_test_sequence(
    label: &str,
    expression: &JsRegExp,
    input: &JsString,
    expected: &[JsValue],
) -> Result<(), String> {
    for (call, step) in expected.iter().enumerate() {
        let call_label = format!("{label}, test call {call}");
        let actual = expression
            .test(input)
            .map_err(|error| format!("{call_label}: {error:?}"))?;
        let expected_result = match object_field(step, "result") {
            JsValue::Bool(value) => value,
            other => return Err(format!("{call_label}: invalid result {other:?}")),
        };
        if actual != expected_result || expression.last_index() != number_field(step, "lastIndex") {
            return Err(format!(
                "{call_label}: expected ({expected_result}, {}), got ({actual}, {})",
                number_field(step, "lastIndex"),
                expression.last_index(),
            ));
        }
    }
    Ok(())
}

#[test]
fn regexp_runtime_matches_all_committed_node_vectors() {
    let vectors = load_vectors();
    assert_eq!(
        vectors.len(),
        235,
        "oracle vector inventory changed unexpectedly"
    );
    let mut failures = Vec::new();

    for entry in &vectors {
        let pattern = string_field(entry, "pattern");
        let flags = string_field(entry, "flags");
        let input = string_field(entry, "input");
        let operation = string_field(entry, "op")
            .to_utf8()
            .expect("oracle operation names must be well-formed UTF-16");
        let expected = object_field(entry, "expected");
        let pattern_text = pattern
            .to_utf8()
            .expect("oracle patterns must be well-formed UTF-16");
        let flags_text = flags
            .to_utf8()
            .expect("oracle flags must be well-formed UTF-16");
        let input_text = input
            .to_utf8()
            .expect("oracle inputs must be well-formed UTF-16");
        let label = format!("/{pattern_text}/{flags_text}/ {operation} on {input_text:?}");
        let expression = match JsRegExp::new(pattern.clone(), flags.clone()) {
            Ok(expression) => expression,
            Err(error) => {
                failures.push(format!("{label}: construction failed: {error:?}"));
                continue;
            }
        };

        let outcome = match operation.as_str() {
            "test" => match expected {
                JsValue::Bool(expected) => expression
                    .test(&input)
                    .map_err(|error| format!("{label}: {error:?}"))
                    .and_then(|actual| {
                        (actual == expected)
                            .then_some(())
                            .ok_or_else(|| format!("{label}: expected {expected}, got {actual}"))
                    }),
                other => Err(format!("{label}: invalid expected value {other:?}")),
            },
            "search" => match expected {
                JsValue::Number(expected) => expression
                    .search(&input)
                    .map_err(|error| format!("{label}: {error:?}"))
                    .and_then(|actual| {
                        assert_eq!(expected.fract(), 0.0, "oracle search index must be integral");
                        let expected = expected.to_isize().expect("oracle search index must fit isize");
                        (actual == expected)
                            .then_some(())
                            .ok_or_else(|| format!("{label}: expected {expected}, got {actual}"))
                    }),
                other => Err(format!("{label}: invalid expected value {other:?}")),
            },
            "replace" => {
                let replacement = string_field(entry, "replacement");
                match expected {
                    JsValue::Utf16String(expected) => expression
                        .replace(&input, &replacement)
                        .map_err(|error| format!("{label}: {error:?}"))
                        .and_then(|actual| {
                            (actual == expected).then_some(()).ok_or_else(|| {
                                format!("{label}: expected {expected:?}, got {actual:?}")
                            })
                        }),
                    other => Err(format!("{label}: invalid expected value {other:?}")),
                }
            }
            "split" => {
                let expected = array_items(&expected)
                    .into_iter()
                    .map(|value| match value {
                        JsValue::Utf16String(value) => Ok(Some(value)),
                        JsValue::Null => Ok(None),
                        other => Err(format!("{label}: invalid split value {other:?}")),
                    })
                    .collect::<Result<Vec<_>, _>>();
                expected.and_then(|expected| {
                    expression
                        .split_all(&input)
                        .map_err(|error| format!("{label}: {error:?}"))
                        .and_then(|actual| {
                            let actual = actual.values();
                            (actual == expected).then_some(()).ok_or_else(|| {
                                format!("{label}: expected {expected:?}, got {actual:?}")
                            })
                        })
                })
            }
            "exec" => compare_exec_sequence(&label, &expression, &input, &array_items(&expected)),
            "test-sequence" => {
                compare_test_sequence(&label, &expression, &input, &array_items(&expected))
            }
            "set-lastindex" => {
                expression.set_last_index(number_field(entry, "setLastIndex"));
                let actual = expression.exec(&input);
                let result = object_field(&expected, "result");
                let comparison = match (result, actual) {
                    (JsValue::Null, Ok(None)) => Ok(()),
                    (JsValue::Utf16String(expected), Ok(Some(actual)))
                        if actual.text() == expected =>
                    {
                        Ok(())
                    }
                    (_, Err(error)) => Err(format!("{label}: {error:?}")),
                    (expected, actual) => {
                        Err(format!("{label}: expected {expected:?}, got {actual:?}"))
                    }
                };
                comparison.and_then(|()| {
                    let expected_last_index = number_field(&expected, "lastIndex");
                    (expression.last_index() == expected_last_index)
                        .then_some(())
                        .ok_or_else(|| {
                            format!(
                                "{label}: expected lastIndex {expected_last_index}, got {}",
                                expression.last_index(),
                            )
                        })
                })
            }
            "match" if flags_text.contains('g') => {
                let expected = match expected {
                    JsValue::Null => None,
                    value => Some(
                        array_items(&value)
                            .into_iter()
                            .map(|item| match item {
                                JsValue::Utf16String(value) => Some(value),
                                other => panic!("{label}: invalid match item {other:?}"),
                            })
                            .collect::<Vec<_>>(),
                    ),
                };
                expression
                    .match_result(&input)
                    .map_err(|error| format!("{label}: {error:?}"))
                    .and_then(|actual| {
                        let actual = actual.map(|array| array.array().values());
                        (actual == expected).then_some(()).ok_or_else(|| {
                            format!("{label}: expected {expected:?}, got {actual:?}")
                        })
                    })
            }
            "match" => match (&expected, expression.match_result(&input)) {
                (JsValue::Null, Ok(None)) => Ok(()),
                (JsValue::Object(_), Ok(Some(actual))) => {
                    compare_match_result(&label, &expected_match(&expected), &actual)
                }
                (_, Err(error)) => Err(format!("{label}: {error:?}")),
                (expected, actual) => {
                    Err(format!("{label}: expected {expected:?}, got {actual:?}"))
                }
            },
            "matchAll" if matches!(expected, JsValue::Object(_)) => {
                match expression.match_all_for_string(&input) {
                    Err(error) if error.kind() == JsErrorKind::TypeError => Ok(()),
                    Err(error) => Err(format!("{label}: expected TypeError, got {error:?}")),
                    Ok(_) => Err(format!("{label}: expected TypeError")),
                }
            }
            "matchAll" => match expression.match_all_for_string(&input) {
                Err(error) => Err(format!("{label}: {error:?}")),
                Ok(iterator) => {
                    let actual = iterator.collect::<Result<Vec<_>, _>>();
                    match actual {
                        Err(error) => Err(format!("{label}: {error:?}")),
                        Ok(actual) => {
                            let expected = array_items(&expected)
                                .iter()
                                .map(expected_match)
                                .collect::<Vec<_>>();
                            if actual.len() != expected.len() {
                                Err(format!(
                                    "{label}: expected {} matches, got {}",
                                    expected.len(),
                                    actual.len(),
                                ))
                            } else {
                                expected
                                    .iter()
                                    .zip(&actual)
                                    .try_for_each(|(expected, actual)| {
                                        compare_exec_match(&label, expected, actual)
                                    })
                            }
                        }
                    }
                }
            },
            other => Err(format!("{label}: unknown operation {other}")),
        };
        if let Err(error) = outcome {
            failures.push(error);
        }
    }

    assert!(
        failures.is_empty(),
        "{} Node oracle vector(s) diverged:\n - {}",
        failures.len(),
        failures.join("\n - "),
    );
}
