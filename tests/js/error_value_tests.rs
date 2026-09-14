use tsonic_rust_js::equality::JsStrictEqual;
use tsonic_rust_js::value::JsValue;
use tsonic_rust_runtime::{JsError, JsErrorKind};

#[test]
fn error_values_preserve_kind_message_and_reference_identity() {
    let error = JsError::new(JsErrorKind::RangeError, "bounds");
    let value = JsValue::from_error(&error);
    let alias = value.clone();
    assert!(value.is_error());
    assert!(alias.is_error_kind(JsErrorKind::RangeError));
    assert!(!alias.is_error_kind(JsErrorKind::TypeError));
    assert_eq!(alias.error_value().message(), "bounds");
    assert!(error.strict_equal(&alias.error_value()));
    assert!(value.strict_equal(&JsValue::from_error(&error)));
    assert!(!value.strict_equal(&JsValue::from_error(&JsError::new(JsErrorKind::RangeError, "bounds"))));
}

#[test]
fn non_error_values_cannot_pass_error_tests_or_projections() {
    for value in [JsValue::undefined(), JsValue::null(), JsValue::Number(1.0), JsValue::object(tsonic_rust_js::object::JsObject::new())] {
        assert!(!value.is_error());
        assert!(!value.is_error_kind(JsErrorKind::Error));
    }
    assert!(std::panic::catch_unwind(|| JsValue::Number(1.0).error_value()).is_err());
}
