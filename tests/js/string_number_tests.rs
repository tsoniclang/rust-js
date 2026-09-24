use tsonic_rust_js::{JsArray, JsStringNumber};

#[test]
fn string_number_arrays_keep_native_strings_and_live_aliases() {
    let values = JsArray::from_dense(vec![JsStringNumber::from_string("😀".to_owned())]);
    let alias = values.clone();
    assert_eq!(
        alias.get(0).unwrap().as_string().as_bytes(),
        &[240, 159, 152, 128]
    );
    values.set(0, JsStringNumber::Number(4.0));
    assert_eq!(alias.get(0).unwrap().as_number(), 4.0);
    assert!(values.get(1).is_none());
    assert_eq!(values.get(0).unwrap().type_of(), "number");
    assert_eq!(JsStringNumber::String("x".to_owned()).type_of(), "string");
    assert_ne!(
        JsStringNumber::Number(f64::NAN),
        JsStringNumber::Number(f64::NAN)
    );
    assert_eq!(JsStringNumber::Number(0.0), JsStringNumber::Number(-0.0));
    assert_ne!(
        JsStringNumber::String("4".to_owned()),
        JsStringNumber::Number(4.0)
    );
}

#[test]
fn native_options_preserve_absence_without_losing_values_or_membership() {
    let values = JsArray::from_dense(vec![
        None,
        Some(JsStringNumber::Number(0.0)),
        Some(JsStringNumber::from_string(String::new())),
    ]);
    let alias = values.clone();
    assert_eq!(values.get(0), Some(None));
    assert!(values.has_index(0));
    assert!(!values.has_index(3));
    assert_eq!(values.get(3), None);
    assert_eq!(values.get(1).unwrap().unwrap().as_number(), 0.0);
    assert_eq!(values.get(2).unwrap().unwrap().as_string(), "");
    alias.set(1, None);
    assert_eq!(values.get(1), Some(None));
    assert!(std::panic::catch_unwind(|| JsStringNumber::Number(0.0).as_string()).is_err());
    assert!(
        std::panic::catch_unwind(|| JsStringNumber::from_string(String::new()).as_number())
            .is_err()
    );
}
