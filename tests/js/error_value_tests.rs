use tsonic_rust_js::equality::JsStrictEqual;
use tsonic_rust_js::value::JsValue;
use tsonic_rust_runtime::{JsError, JsErrorKind, ToSourceString};

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
    assert!(!value.strict_equal(&JsValue::from_error(&JsError::new(
        JsErrorKind::RangeError,
        "bounds"
    ))));
    assert_eq!(error, JsError::new(JsErrorKind::RangeError, "bounds"));
    assert_ne!(error, JsError::new(JsErrorKind::TypeError, "bounds"));
}

#[cfg(target_has_atomic = "ptr")]
#[test]
fn native_diagnostic_errors_retain_send_and_sync() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<JsError>();
    require_send_sync::<tsonic_rust_runtime::TsonicError>();
}

#[test]
fn non_error_values_cannot_pass_error_tests_or_projections() {
    for value in [
        JsValue::undefined(),
        JsValue::null(),
        JsValue::Number(1.0),
        JsValue::object(tsonic_rust_js::object::JsObject::new()),
    ] {
        assert!(!value.is_error());
        assert!(!value.is_error_kind(JsErrorKind::Error));
    }
    assert!(std::panic::catch_unwind(|| JsValue::Number(1.0).error_value()).is_err());
}

#[test]
fn source_error_display_and_json_do_not_expose_diagnostic_storage() {
    let empty = JsError::error("");
    assert_eq!(empty.to_source_string(), "Error");
    assert_eq!(empty.to_string(), "Error: ");
    let JsValue::Closed(value) = JsValue::from_error(&empty) else {
        panic!("missing closed error")
    };
    assert_eq!(value.inspect(), "Error");
    let JsValue::Object(object) = value.project_json().expect("error JSON projection") else {
        panic!("non-object error JSON")
    };
    assert!(object.borrow().keys_exact().is_empty());
}
