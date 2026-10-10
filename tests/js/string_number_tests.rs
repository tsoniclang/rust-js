use tsonic_rust_js::{JsArray, JsStringNumber};

#[test]
fn native_conversion_traits_forward_borrowed_and_optional_values() {
    use tsonic_rust_js::abi::{number_from_value, string_from_value};

    let text = String::from("12.5");
    let view = text.as_str();
    assert_eq!(string_from_value(&view), "12.5");
    assert_eq!(string_from_value(&Some(view)), "12.5");
    assert_eq!(string_from_value(&None::<&str>), "null");
    assert_eq!(string_from_value(&Some(None::<&str>)), "null");
    assert_eq!(number_from_value(&view), 12.5);
    assert_eq!(number_from_value(&Some(view)), 12.5);
    assert_eq!(number_from_value(&None::<&str>), 0.0);
    assert_eq!(number_from_value(&Some(None::<&str>)), 0.0);
    assert_eq!(number_from_value(&&42_i64), 42.0);
}

#[test]
fn borrowed_string_writing_forwards_the_referent_without_owned_temporaries() {
    use tsonic_rust_js::string::JsToString;

    struct DirectWriter;

    impl JsToString for DirectWriter {
        fn to_js_string(&self) -> String {
            panic!("borrowed writing must not request an owned string")
        }

        fn write_js_string(&self, output: &mut String) {
            output.push_str("native");
        }

        fn write_js_join_value(&self, output: &mut String) {
            output.push_str("join");
        }
    }

    let writer = DirectWriter;
    let reference = &writer;
    let mut output = String::with_capacity(32);
    <&DirectWriter as JsToString>::write_js_string(&reference, &mut output);
    <&DirectWriter as JsToString>::write_js_join_value(&reference, &mut output);
    assert_eq!(output, "nativejoin");
}

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
