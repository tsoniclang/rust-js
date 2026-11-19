use super::JsValue;
use crate::equality::JsHash;
use crate::number::NumericRef;
use std::cmp::Ordering;

fn signed(value: f64) -> Option<i64> {
    (value >= i64::MIN as f64 && value < -(i64::MIN as f64) && value.fract() == 0.0)
        .then_some(value as i64)
}

fn unsigned(value: f64) -> Option<u64> {
    (value >= 0.0 && value < (u64::MAX as u128 + 1) as f64 && value.fract() == 0.0)
        .then_some(value as u64)
}

pub(super) fn equal(left: &JsValue, right: &JsValue, signed_zero: bool, equal_nan: bool) -> bool {
    let (Some(left), Some(right)) = (left.numeric_ref(), right.numeric_ref()) else {
        return false;
    };
    if let (NumericRef::Float(left), NumericRef::Float(right)) = (left, right) {
        if left.is_nan() && right.is_nan() {
            return equal_nan;
        }
        if signed_zero && left == 0.0 && right == 0.0 {
            return left.is_sign_negative() == right.is_sign_negative();
        }
    } else if signed_zero && [left, right].iter().any(|value|
        matches!(value, NumericRef::Float(value) if *value == 0.0 && value.is_sign_negative())) {
        return false;
    }
    left.compare(right) == Some(Ordering::Equal)
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
