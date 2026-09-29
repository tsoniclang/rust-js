use crate::value::JsValue;

pub fn is_nan(value: &JsValue) -> bool {
    to_number(value).is_nan()
}

pub fn is_finite(value: &JsValue) -> bool {
    to_number(value).is_finite()
}

pub fn to_number(value: &JsValue) -> f64 {
    match value {
        JsValue::Null => 0.0,
        JsValue::Bool(value) => {
            if *value {
                1.0
            } else {
                0.0
            }
        }
        JsValue::Number(value) => *value,
        JsValue::Integer(value) => *value as f64,
        JsValue::UnsignedInteger(value) => *value as f64,
        JsValue::Int8(value) => f64::from(*value),
        JsValue::Uint8(value) => f64::from(*value),
        JsValue::Int16(value) => f64::from(*value),
        JsValue::Uint16(value) => f64::from(*value),
        JsValue::Int32(value) => f64::from(*value),
        JsValue::Uint32(value) => f64::from(*value),
        JsValue::NativeInt(value) => *value as f64,
        JsValue::NativeUint(value) => *value as f64,
        JsValue::Float32(value) => f64::from(*value),
        JsValue::String(value) => crate::number::numeric_string(value),
        JsValue::Utf16String(value) => {
            let Ok(text) = value.to_utf8() else {
                return f64::NAN;
            };
            crate::number::numeric_string(&text)
        }
        JsValue::Symbol(_)
        | JsValue::Object(_)
        | JsValue::Array(_)
        | JsValue::Closed(_)
        | JsValue::JsonProjection(_) => f64::NAN,
    }
}
