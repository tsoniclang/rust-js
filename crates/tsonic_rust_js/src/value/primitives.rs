use super::JsValue;
use crate::number::NumericRef;

macro_rules! impl_native_number_from {
    ($($variant:ident: $carrier:ty),+ $(,)?) => {
        $(impl From<$carrier> for JsValue {
            fn from(value: $carrier) -> Self {
                Self::$variant(value)
            }
        })+
    };
}

impl_native_number_from!(
    Integer: i64,
    UnsignedInteger: u64,
    Number: f64,
    Int8: i8,
    Uint8: u8,
    Int16: i16,
    Uint16: u16,
    Int32: i32,
    Uint32: u32,
    NativeInt: isize,
    NativeUint: usize,
    Float32: f32,
);

impl JsValue {
    pub fn numeric_ref(&self) -> Option<NumericRef<'_>> {
        match self {
            Self::Number(value) => Some(NumericRef::Float(*value)),
            Self::Integer(value) => Some(NumericRef::Signed(i128::from(*value))),
            Self::UnsignedInteger(value) => Some(NumericRef::Unsigned(u128::from(*value))),
            Self::Int8(value) => Some(NumericRef::Signed(i128::from(*value))),
            Self::Uint8(value) => Some(NumericRef::Unsigned(u128::from(*value))),
            Self::Int16(value) => Some(NumericRef::Signed(i128::from(*value))),
            Self::Uint16(value) => Some(NumericRef::Unsigned(u128::from(*value))),
            Self::Int32(value) => Some(NumericRef::Signed(i128::from(*value))),
            Self::Uint32(value) => Some(NumericRef::Unsigned(u128::from(*value))),
            Self::NativeInt(value) => Some(NumericRef::Signed(*value as i128)),
            Self::NativeUint(value) => Some(NumericRef::Unsigned(*value as u128)),
            Self::Float32(value) => Some(NumericRef::Float(f64::from(*value))),
            _ => None,
        }
    }

    pub fn type_of(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Number(_)
            | Self::Float32(_)
            | Self::Int8(_)
            | Self::Uint8(_)
            | Self::Int16(_)
            | Self::Uint16(_)
            | Self::Int32(_)
            | Self::Uint32(_)
            | Self::NativeInt(_)
            | Self::NativeUint(_) => "number",
            Self::Integer(_) | Self::UnsignedInteger(_) => "bigint",
            Self::String(_) => "string",
            Self::Symbol(_) => "symbol",
            Self::Null
            | Self::Utf16String(_)
            | Self::Object(_)
            | Self::Record(_)
            | Self::Array(_)
            | Self::Closed(_)
            | Self::JsonProjection(_) => "object",
        }
    }
}
