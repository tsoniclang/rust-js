use crate::errors::JsResult;
use num_traits::{FromPrimitive, ToPrimitive};
use std::cmp::Ordering;
use tsonic_rust_runtime::BigInt;

#[derive(Clone, Debug)]
pub enum JsNumeric {
    Number(f64),
    BigInt(BigInt),
}

impl JsNumeric {
    pub fn from_number(value: f64) -> Self {
        Self::Number(value)
    }
    pub fn from_int32(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
    pub fn from_bigint(value: &BigInt) -> Self {
        Self::BigInt(value.clone())
    }

    pub fn type_of(&self) -> String {
        match self {
            Self::Number(_) => "number",
            Self::BigInt(_) => "bigint",
        }
        .to_owned()
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Self::Number(value) => *value,
            Self::BigInt(_) => unreachable!("checked numeric refinement must select number"),
        }
    }

    pub fn as_bigint(&self) -> BigInt {
        match self {
            Self::BigInt(value) => value.clone(),
            Self::Number(_) => unreachable!("checked numeric refinement must select bigint"),
        }
    }

    pub fn to_bigint(&self) -> JsResult<BigInt> {
        match self {
            Self::Number(value) => crate::bigint::from_number(*value),
            Self::BigInt(value) => Ok(value.clone()),
        }
    }

    pub fn to_number(&self) -> f64 {
        match self {
            Self::Number(value) => *value,
            Self::BigInt(value) => bigint_to_number(value),
        }
    }

    pub fn loose_equal(&self, other: &Self) -> bool {
        self.compare(other) == Some(Ordering::Equal)
    }

    pub fn loose_not_equal(&self, other: &Self) -> bool {
        !self.loose_equal(other)
    }
    pub fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
    pub fn strict_not_equal(&self, other: &Self) -> bool {
        self != other
    }
    pub fn less_than(&self, other: &Self) -> bool {
        self.compare(other) == Some(Ordering::Less)
    }
    pub fn greater_than(&self, other: &Self) -> bool {
        self.compare(other) == Some(Ordering::Greater)
    }
    pub fn less_than_or_equal(&self, other: &Self) -> bool {
        matches!(self.compare(other), Some(Ordering::Less | Ordering::Equal))
    }
    pub fn greater_than_or_equal(&self, other: &Self) -> bool {
        matches!(
            self.compare(other),
            Some(Ordering::Greater | Ordering::Equal)
        )
    }

    fn compare(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::BigInt(left), Self::BigInt(right)) => Some(left.cmp(right)),
            (Self::BigInt(left), Self::Number(right)) => compare_bigint_number(left, *right),
            (Self::Number(left), Self::BigInt(right)) => {
                compare_bigint_number(right, *left).map(Ordering::reverse)
            }
        }
    }
}

pub fn bigint_to_number(value: &BigInt) -> f64 {
    let integer = num_bigint::BigInt::from_signed_bytes_le(&value.to_signed_bytes_le());
    integer.to_f64().unwrap_or_else(|| {
        if integer.sign() == num_bigint::Sign::Minus {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

fn compare_bigint_number(left: &BigInt, right: f64) -> Option<Ordering> {
    if right.is_nan() {
        return None;
    }
    if right == f64::INFINITY {
        return Some(Ordering::Less);
    }
    if right == f64::NEG_INFINITY {
        return Some(Ordering::Greater);
    }
    let integer = num_bigint::BigInt::from_signed_bytes_le(&left.to_signed_bytes_le());
    let truncated = num_bigint::BigInt::from_f64(right)?;
    let ordering = integer.cmp(&truncated);
    Some(if ordering == Ordering::Equal && right.fract() != 0.0 {
        if right.is_sign_negative() {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    } else {
        ordering
    })
}

impl PartialEq for JsNumeric {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::BigInt(left), Self::BigInt(right)) => left == right,
            _ => false,
        }
    }
}
