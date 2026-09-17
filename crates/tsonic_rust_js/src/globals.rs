use crate::value::JsValue;

pub fn is_nan(value: &JsValue) -> bool {
    to_number(value).is_nan()
}

pub fn is_finite(value: &JsValue) -> bool {
    to_number(value).is_finite()
}

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
        JsValue::String(value) => {
            let Ok(text) = value.to_utf8() else {
                return f64::NAN;
            };
            let trimmed = text.trim_matches(is_ecmascript_whitespace);
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        JsValue::Symbol(_)
        | JsValue::Object(_)
        | JsValue::Array(_)
        | JsValue::Closed(_)
        | JsValue::JsonProjection(_) => f64::NAN,
    }
}

pub(crate) fn is_ecmascript_whitespace(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}
