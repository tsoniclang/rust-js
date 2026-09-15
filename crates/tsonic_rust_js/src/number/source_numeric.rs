use super::JsNumeric;
use crate::errors::JsResult;
use tsonic_rust_runtime::BigInt;

pub trait SourceNumeric {
    fn source_numeric(&self) -> JsNumeric;

    fn to_number(&self) -> f64 {
        self.source_numeric().to_number()
    }

    fn to_bigint(&self) -> JsResult<BigInt> {
        self.source_numeric().to_bigint()
    }

    fn less_than(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().less_than(&other.source_numeric())
    }

    fn less_than_or_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric()
            .less_than_or_equal(&other.source_numeric())
    }

    fn greater_than(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().greater_than(&other.source_numeric())
    }

    fn greater_than_or_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric()
            .greater_than_or_equal(&other.source_numeric())
    }

    fn strict_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().strict_equal(&other.source_numeric())
    }

    fn strict_not_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric()
            .strict_not_equal(&other.source_numeric())
    }

    fn loose_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric().loose_equal(&other.source_numeric())
    }

    fn loose_not_equal(&self, other: &impl SourceNumeric) -> bool {
        self.source_numeric()
            .loose_not_equal(&other.source_numeric())
    }
}

impl SourceNumeric for JsNumeric {
    fn source_numeric(&self) -> JsNumeric {
        self.clone()
    }
}

impl SourceNumeric for BigInt {
    fn source_numeric(&self) -> JsNumeric {
        JsNumeric::from_bigint(self)
    }
}

macro_rules! number_sources {
    ($($scalar:ty),+ $(,)?) => { $(
        impl SourceNumeric for $scalar {
            fn source_numeric(&self) -> JsNumeric {
                JsNumeric::from_number(f64::from(*self))
            }
        }
    )+ };
}

number_sources!(f32, f64, i8, i16, i32, u8, u16, u32);

macro_rules! bigint_sources {
    ($($scalar:ty),+ $(,)?) => { $(
        impl SourceNumeric for $scalar {
            fn source_numeric(&self) -> JsNumeric {
                let bytes = i128::from(*self).to_le_bytes();
                JsNumeric::BigInt(BigInt::from_signed_bytes_le(&bytes))
            }
        }
    )+ };
}

bigint_sources!(i64, u64, i128);

impl SourceNumeric for u128 {
    fn source_numeric(&self) -> JsNumeric {
        let mut bytes = [0; 17];
        bytes[..16].copy_from_slice(&self.to_le_bytes());
        JsNumeric::BigInt(BigInt::from_signed_bytes_le(&bytes))
    }
}
