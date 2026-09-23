use super::JsValue;
use crate::equality::JsHash;

fn signed(value: f64) -> Option<i64> {
    (value >= i64::MIN as f64 && value < -(i64::MIN as f64) && value.fract() == 0.0)
        .then_some(value as i64)
}

fn unsigned(value: f64) -> Option<u64> {
    (value >= 0.0 && value < (u64::MAX as u128 + 1) as f64 && value.fract() == 0.0)
        .then_some(value as u64)
}

pub(super) fn integer_equal(left: &JsValue, right: &JsValue, signed_zero: bool) -> bool {
    match (left, right) {
        (JsValue::Integer(left), JsValue::Integer(right)) => left == right,
        (JsValue::UnsignedInteger(left), JsValue::UnsignedInteger(right)) => left == right,
        (JsValue::Integer(signed), JsValue::UnsignedInteger(unsigned))
        | (JsValue::UnsignedInteger(unsigned), JsValue::Integer(signed)) => {
            u64::try_from(*signed).ok() == Some(*unsigned)
        }
        (JsValue::Integer(integer), JsValue::Number(number))
        | (JsValue::Number(number), JsValue::Integer(integer)) => {
            !(signed_zero && number.is_sign_negative() && *number == 0.0)
                && signed(*number) == Some(*integer)
        }
        (JsValue::UnsignedInteger(integer), JsValue::Number(number))
        | (JsValue::Number(number), JsValue::UnsignedInteger(integer)) => {
            !(signed_zero && number.is_sign_negative() && *number == 0.0)
                && unsigned(*number) == Some(*integer)
        }
        _ => false,
    }
}

pub(super) fn float_hash(value: f64) -> u64 {
    if let Some(integer) = unsigned(value) {
        integer.js_hash()
    } else if let Some(integer) = signed(value) {
        integer.js_hash()
    } else {
        value.js_hash()
    }
}
