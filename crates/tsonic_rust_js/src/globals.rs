use crate::value::JsValue;

pub fn to_number(value: &JsValue) -> f64 {
    match value {
        JsValue::Undefined => f64::NAN,
        JsValue::Null => 0.0,
        JsValue::Bool(value) => {
            if *value {
                1.0
            } else {
                0.0
            }
        }
        JsValue::Number(value) => *value,
        JsValue::String(value) => crate::number::parse_float(value),
        JsValue::Utf16String(value) => {
            let Ok(text) = value.to_utf8() else {
                return f64::NAN;
            };
            crate::number::parse_float(&text)
        }
        JsValue::Symbol(_)
        | JsValue::Object(_)
        | JsValue::Array(_)
        | JsValue::Closed(_)
        | JsValue::JsonProjection(_) => f64::NAN,
    }
}
