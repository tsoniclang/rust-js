use tsonic_rust_js::equality::JsStrictEqual;
use tsonic_rust_js::value::JsValue;
use tsonic_rust_runtime::{
    ErrorObject, JsError, JsErrorKind, MutableJsError, ToSourceString, TsonicError,
    WritableErrorObject, WritableRetainedError,
};

#[test]
fn error_values_preserve_kind_message_and_reference_identity() {
    let error = JsError::new(JsErrorKind::RangeError, "bounds");
    let value = JsValue::from_error(error.clone());
    let alias = value.clone();
    assert!(value.is_error());
    assert!(alias.is_error_kind(JsErrorKind::RangeError));
    assert!(!alias.is_error_kind(JsErrorKind::TypeError));
    assert_eq!(alias.error_value().error_message(), "bounds");
    assert_eq!(
        error.identity_key(),
        alias.error_value().error_identity_key()
    );
    assert!(value.strict_equal(&JsValue::from_error(error.clone())));
    assert!(!value.strict_equal(&JsValue::from_error(JsError::new(
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
fn closed_errors_keep_live_writable_fields_and_identity_across_aliases() {
    let original = MutableJsError::error("before");
    let alias = original.clone();
    let value = JsValue::from_error(original);
    let retained = value.clone();
    let writable = WritableRetainedError::try_from(value.into_error().unwrap()).unwrap();
    writable.set_error_name(String::from("ChangedError"));
    writable.set_error_message(String::from("after"));
    writable.set_error_stack(Some(String::from("explicit stack")));
    assert!(retained.strict_equal(&JsValue::from_error(alias.clone())));
    assert_eq!(
        retained.as_error().unwrap().error_identity_key(),
        alias.identity_key()
    );
    assert_eq!(retained.error_value().error_message(), "after");
    assert_eq!(
        retained.error_value().error_stack().unwrap(),
        "explicit stack"
    );
    assert_eq!(
        tsonic_rust_js::value::closed_value_string(&retained).unwrap(),
        "ChangedError: after"
    );
}

#[test]
fn consuming_non_error_projection_retains_width_buffer_and_one_absence_state() {
    for value in [
        JsValue::null(),
        JsValue::undefined(),
        JsValue::Bool(false),
        JsValue::Integer(i64::MIN),
        JsValue::UnsignedInteger(u64::MAX),
        JsValue::UnsignedInteger(9_007_199_254_740_993),
    ] {
        let alias = value.clone();
        let returned = value.into_error().unwrap_err();
        assert!(returned.strict_equal(&alias));
        assert!(returned.as_error().is_none());
    }
    let text = String::from("native move");
    let pointer = text.as_ptr();
    let JsValue::String(returned) = JsValue::String(text).into_error().unwrap_err() else {
        panic!("original string carrier was changed");
    };
    assert_eq!(returned.as_ptr(), pointer);
    assert!(JsValue::null()
        .into_error()
        .unwrap_err()
        .strict_equal(&JsValue::undefined()));
}

#[test]
fn native_context_payload_is_retained_without_inventing_a_json_projection() {
    let source = JsError::error("native context");
    let identity = source.error_identity_key();
    let value = JsValue::from_error(TsonicError::Node {
        code: String::from("EXACT_CODE"),
        source,
    });
    let JsValue::Closed(payload) = &value else {
        panic!("native context was not retained");
    };
    assert!(payload.project_json().is_err());
    assert_eq!(value.as_error().unwrap().error_identity_key(), identity);
    let tsonic_rust_runtime::RetainedError::Runtime(context) = value.into_error().unwrap() else {
        panic!("native context was narrowed to a message");
    };
    let TsonicError::Node { code, .. } = context.as_ref() else {
        panic!("native context kind was lost");
    };
    assert_eq!(code, "EXACT_CODE");
}

#[test]
fn source_error_display_and_json_do_not_expose_diagnostic_storage() {
    let empty = JsError::error("");
    assert_eq!(empty.to_source_string(), "Error");
    assert_eq!(empty.to_string(), "Error: ");
    let JsValue::Closed(value) = JsValue::from_error(empty.clone()) else {
        panic!("missing closed error")
    };
    assert_eq!(value.inspect(), "Error");
    let JsValue::Object(object) = value.project_json().expect("error JSON projection") else {
        panic!("non-object error JSON")
    };
    assert!(object.borrow().keys_exact().is_empty());
}
