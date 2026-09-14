use tsonic_rust_js::{JsArray, JsStringNumber};
use tsonic_rust_runtime::{Null, Undefined};

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
    alias.push(JsStringNumber::Undefined);
    alias.push(JsStringNumber::Null);
    assert_eq!(values.get(1).unwrap(), Undefined);
    assert_eq!(values.get(2).unwrap(), Null);
    assert_ne!(values.get(1).unwrap(), values.get(2).unwrap());
    assert!(values.get(3).is_none());
    assert_eq!(values.get(1).unwrap().type_of(), "undefined");
    assert_eq!(values.get(2).unwrap().type_of(), "object");
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
fn nullish_refinements_preserve_the_exact_selected_variant() {
    assert_eq!(JsStringNumber::Null.as_null(), Null);
    assert_eq!(JsStringNumber::Undefined.as_undefined(), Undefined);
    for value in [
        JsStringNumber::Number(0.0),
        JsStringNumber::String(String::new()),
        JsStringNumber::Undefined,
    ] {
        assert!(std::panic::catch_unwind(|| value.as_null()).is_err());
    }
    for value in [
        JsStringNumber::Number(0.0),
        JsStringNumber::String(String::new()),
        JsStringNumber::Null,
    ] {
        assert!(std::panic::catch_unwind(|| value.as_undefined()).is_err());
    }
}
