use std::cell::Cell;
use std::rc::Rc;
use tsonic_rust_js::{abi, value::JsonProjection, JsArray, JsErrorKind, JsValue};
use tsonic_rust_runtime::{EmptyObject, JsError};

#[test]
fn closed_strings_preserve_native_values_without_inspector_quoting() {
    for (value, expected) in [
        (JsValue::Null, "null"),
        (JsValue::Bool(false), "false"),
        (JsValue::Number(-0.0), "0"),
        (JsValue::Integer(9_007_199_254_740_993), "9007199254740993"),
        (JsValue::UnsignedInteger(u64::MAX), "18446744073709551615"),
        (JsValue::String("text\n😀".to_owned()), "text\n😀"),
        (JsValue::closed(EmptyObject::new()), "[object Object]"),
        (JsValue::from_error(JsError::error("")), "Error"),
        (
            JsValue::from_error(JsError::error("failure")),
            "Error: failure",
        ),
    ] {
        assert_eq!(abi::closed_value_string(&value).unwrap(), expected);
    }
    let repeated = JsValue::array(JsArray::from_dense(vec![
        JsValue::String("x".to_owned()),
        JsValue::Null,
    ]));
    let values = JsValue::array(JsArray::from_dense(vec![
        repeated.clone(),
        repeated,
        JsValue::Integer(42),
    ]));
    assert_eq!(abi::closed_value_string(&values).unwrap(), "x,,x,,42");
}

#[test]
fn closed_strings_reject_cycles_and_do_not_invoke_json_only_callbacks() {
    let values = JsArray::new();
    values.push(JsValue::array(values.clone()));
    let cycle = JsValue::array(values.clone());
    assert_eq!(
        abi::closed_value_string(&cycle).unwrap_err().kind(),
        JsErrorKind::TypeError
    );
    values.set_len(0_usize);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let value = JsValue::JsonProjection(JsonProjection::new((), move |_, _| {
        observed.set(observed.get() + 1);
        Ok(JsValue::String("not a String conversion".to_owned()))
    }));
    assert_eq!(
        abi::closed_value_string(&value).unwrap_err().kind(),
        JsErrorKind::Unsupported
    );
    assert_eq!(calls.get(), 0);
}
