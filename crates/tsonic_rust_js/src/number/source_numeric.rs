use std::cmp::Ordering;

use super::{bigint_to_number, JsNumeric, NumericRef};
use crate::errors::JsResult;
use tsonic_rust_runtime::BigInt;

pub trait SourceNumeric {
    fn source_numeric(&self) -> NumericRef<'_>;
    fn bigint_domain(&self) -> bool;

    fn to_number(&self) -> f64 {
        match self.source_numeric() {
            NumericRef::Float(value) => value,
            NumericRef::Signed(value) => value as f64,
            NumericRef::Unsigned(value) => value as f64,
            NumericRef::BigInt(value) => bigint_to_number(value),
        }
    }

    fn to_bigint(&self) -> JsResult<BigInt> {
        match self.source_numeric() {
            NumericRef::Float(value) => crate::bigint::from_number(value),
            NumericRef::Signed(value) => Ok(crate::bigint::from_integer(value)),
            NumericRef::Unsigned(value) => Ok(crate::bigint::from_integer(value)),
            NumericRef::BigInt(value) => Ok(value.clone()),
        }
    }

    fn less_than(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().compare(other.source_numeric()) == Some(Ordering::Less)
    }

    fn less_than_or_equal(&self, other: &impl SourceNumeric) -> bool {
        matches!(self.source_numeric().compare(other.source_numeric()), Some(Ordering::Less | Ordering::Equal))
    }

    fn greater_than(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().compare(other.source_numeric()) == Some(Ordering::Greater)
    }

    fn greater_than_or_equal(&self, other: &impl SourceNumeric) -> bool {
        matches!(self.source_numeric().compare(other.source_numeric()), Some(Ordering::Greater | Ordering::Equal))
    }

    fn strict_equal(&self, other: &impl SourceNumeric) -> bool {
        self.bigint_domain() == other.bigint_domain() && self.loose_equal(other)
    }

    fn strict_not_equal(&self, other: &impl SourceNumeric) -> bool { !self.strict_equal(other) }

    fn loose_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().compare(other.source_numeric()) == Some(Ordering::Equal)
    }

    fn loose_not_equal(&self, other: &impl SourceNumeric) -> bool { !self.loose_equal(other) }
}

impl SourceNumeric for JsNumeric {
    fn source_numeric(&self) -> NumericRef<'_> {
        match self {
            Self::Number(value) => NumericRef::Float(*value),
            Self::BigInt(value) => NumericRef::BigInt(value),
        }
    }
    fn bigint_domain(&self) -> bool { matches!(self, Self::BigInt(_)) }
}

impl SourceNumeric for BigInt {
    fn source_numeric(&self) -> NumericRef<'_> { NumericRef::BigInt(self) }
    fn bigint_domain(&self) -> bool { true }
}

macro_rules! numeric_sources {
    ($variant:ident, $storage:ty, $domain:expr, $($scalar:ty),+ $(,)?) => { $(
        impl SourceNumeric for $scalar {
            fn source_numeric(&self) -> NumericRef<'_> { NumericRef::$variant(*self as $storage) }
            fn bigint_domain(&self) -> bool { $domain }
        }
    )+ };
}

numeric_sources!(Float, f64, false, f32, f64);
numeric_sources!(Signed, i128, false, i8, i16, i32);
numeric_sources!(Unsigned, u128, false, u8, u16, u32);
numeric_sources!(Signed, i128, true, i64, i128);
numeric_sources!(Unsigned, u128, true, u64, u128);
