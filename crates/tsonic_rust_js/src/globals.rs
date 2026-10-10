use crate::value::JsValue;

pub trait NumberInput {
    fn to_number(&self) -> f64;
}

impl<Value: NumberInput + ?Sized> NumberInput for &Value {
    #[inline]
    fn to_number(&self) -> f64 {
        Value::to_number(*self)
    }
}

#[inline]
pub fn number_from_value<Value: NumberInput + ?Sized>(value: &Value) -> f64 {
    value.to_number()
}

macro_rules! native_number_input {
    ($($carrier:ty),+ $(,)?) => {$(
        impl NumberInput for $carrier {
            #[inline]
            fn to_number(&self) -> f64 { *self as f64 }
        }
    )+};
}

native_number_input!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize, f32, f64);

impl NumberInput for bool {
    #[inline]
    fn to_number(&self) -> f64 {
        f64::from(u8::from(*self))
    }
}

impl NumberInput for () {
    #[inline]
    fn to_number(&self) -> f64 {
        0.0
    }
}

impl<Value: NumberInput> NumberInput for Option<Value> {
    #[inline]
    fn to_number(&self) -> f64 {
        self.as_ref().map_or(0.0, NumberInput::to_number)
    }
}

impl NumberInput for str {
    fn to_number(&self) -> f64 {
        crate::number::numeric_string(self)
    }
}

impl NumberInput for String {
    fn to_number(&self) -> f64 {
        self.as_str().to_number()
    }
}

impl NumberInput for tsonic_rust_runtime::BigInt {
    fn to_number(&self) -> f64 {
        crate::number::bigint_to_number(self)
    }
}

impl NumberInput for crate::number::JsNumeric {
    fn to_number(&self) -> f64 {
        Self::to_number(self)
    }
}

impl NumberInput for JsValue {
    fn to_number(&self) -> f64 {
        to_number(self)
    }
}

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
        | JsValue::Record(_)
        | JsValue::Array(_)
        | JsValue::Closed(_)
        | JsValue::JsonProjection(_) => f64::NAN,
    }
}
