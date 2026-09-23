use crate::native_integer::Integer32;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl fmt::Display for TypedArrayKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Int8 => "Int8",
            Self::Uint8 => "Uint8",
            Self::Uint8Clamped => "Uint8Clamped",
            Self::Int16 => "Int16",
            Self::Uint16 => "Uint16",
            Self::Int32 => "Int32",
            Self::Uint32 => "Uint32",
            Self::Float32 => "Float32",
            Self::Float64 => "Float64",
        })
    }
}

pub trait TypedElement: Copy + Default + fmt::Display + 'static {
    type Value: Copy;
    const KIND: TypedArrayKind;
    const BYTES_PER_ELEMENT: usize;

    fn from_number(value: f64) -> Self;
    fn to_number(self) -> f64;
    fn into_value(self) -> Self::Value;
    fn compare(self, other: Self) -> std::cmp::Ordering;
    fn write_bytes(self, output: &mut [u8]);
    fn read_bytes(bytes: &[u8]) -> Self;
}

macro_rules! integer_element {
    ($type:ty, $kind:expr) => {
        impl TypedElement for $type {
            type Value = Self;
            const KIND: TypedArrayKind = $kind;
            const BYTES_PER_ELEMENT: usize = std::mem::size_of::<$type>();

            fn from_number(value: f64) -> Self {
                value.integer32() as $type
            }

            fn to_number(self) -> f64 {
                self as f64
            }

            fn into_value(self) -> Self::Value {
                self
            }

            fn compare(self, other: Self) -> std::cmp::Ordering {
                self.cmp(&other)
            }

            fn write_bytes(self, output: &mut [u8]) {
                output.copy_from_slice(&self.to_le_bytes());
            }

            fn read_bytes(bytes: &[u8]) -> Self {
                let mut slot = [0_u8; std::mem::size_of::<$type>()];
                slot.copy_from_slice(bytes);
                <$type>::from_le_bytes(slot)
            }
        }
    };
}

integer_element!(i8, TypedArrayKind::Int8);
integer_element!(u8, TypedArrayKind::Uint8);
integer_element!(i16, TypedArrayKind::Int16);
integer_element!(u16, TypedArrayKind::Uint16);
integer_element!(i32, TypedArrayKind::Int32);
integer_element!(u32, TypedArrayKind::Uint32);

impl TypedElement for f32 {
    type Value = Self;
    const KIND: TypedArrayKind = TypedArrayKind::Float32;
    const BYTES_PER_ELEMENT: usize = 4;

    fn from_number(value: f64) -> Self {
        value as f32
    }

    fn to_number(self) -> f64 {
        self as f64
    }

    fn into_value(self) -> Self::Value {
        self
    }

    fn compare(self, other: Self) -> std::cmp::Ordering {
        match (self.is_nan(), other.is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => self.total_cmp(&other),
        }
    }

    fn write_bytes(self, output: &mut [u8]) {
        output.copy_from_slice(&self.to_le_bytes());
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        let mut slot = [0_u8; 4];
        slot.copy_from_slice(bytes);
        Self::from_le_bytes(slot)
    }
}

impl TypedElement for f64 {
    type Value = Self;
    const KIND: TypedArrayKind = TypedArrayKind::Float64;
    const BYTES_PER_ELEMENT: usize = 8;

    fn from_number(value: f64) -> Self {
        value
    }

    fn to_number(self) -> f64 {
        self
    }

    fn into_value(self) -> Self::Value {
        self
    }

    fn compare(self, other: Self) -> std::cmp::Ordering {
        match (self.is_nan(), other.is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => self.total_cmp(&other),
        }
    }

    fn write_bytes(self, output: &mut [u8]) {
        output.copy_from_slice(&self.to_le_bytes());
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        let mut slot = [0_u8; 8];
        slot.copy_from_slice(bytes);
        Self::from_le_bytes(slot)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClampedU8(pub u8);

impl fmt::Display for ClampedU8 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TypedElement for ClampedU8 {
    type Value = u8;
    const KIND: TypedArrayKind = TypedArrayKind::Uint8Clamped;
    const BYTES_PER_ELEMENT: usize = 1;

    fn from_number(value: f64) -> Self {
        Self(to_uint8_clamp(value))
    }

    fn to_number(self) -> f64 {
        self.0 as f64
    }

    fn into_value(self) -> Self::Value {
        self.0
    }

    fn compare(self, other: Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }

    fn write_bytes(self, output: &mut [u8]) {
        output[0] = self.0;
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0])
    }
}

fn to_uint8_clamp(value: f64) -> u8 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    if value >= 255.0 {
        return 255;
    }
    let floor = value.floor();
    let fraction = value - floor;
    if fraction > 0.5 || fraction == 0.5 && (floor as u8) % 2 == 1 {
        floor as u8 + 1
    } else {
        floor as u8
    }
}

pub trait ConvertElement<Target> {
    fn convert_element(self) -> Target;
}

impl<Target> ConvertElement<Target> for ClampedU8
where
    u8: ConvertElement<Target>,
{
    fn convert_element(self) -> Target {
        self.0.convert_element()
    }
}

macro_rules! native_conversion {
    ($source:ty; $($target:ty),+ $(,)?) => {$(
        impl ConvertElement<$target> for $source {
            #[inline]
            fn convert_element(self) -> $target { self as $target }
        }
    )+};
}

macro_rules! integer_conversion {
    ($($source:ty),+ $(,)?) => {$(
        native_conversion!($source; i8, u8, i16, u16, i32, u32, f32, f64);
        impl ConvertElement<ClampedU8> for $source {
            #[inline]
            fn convert_element(self) -> ClampedU8 {
                ClampedU8(self.clamp(0, 255 as $source) as u8)
            }
        }
    )+};
}

macro_rules! float_integer_conversion {
    ($source:ty; $($target:ty),+ $(,)?) => {$(
        impl ConvertElement<$target> for $source {
            #[inline]
            fn convert_element(self) -> $target { self.integer32() as $target }
        }
    )+};
}

macro_rules! float_conversion {
    ($($source:ty),+ $(,)?) => {$(
        native_conversion!($source; f32, f64);
        float_integer_conversion!($source; i8, u8, i16, u16, i32, u32);
        impl ConvertElement<ClampedU8> for $source {
            #[inline]
            fn convert_element(self) -> ClampedU8 { ClampedU8::from_number(self as f64) }
        }
    )+};
}

integer_conversion!(u8, i16, u16, i32, u32, i64, u64, isize, usize, i128, u128);
native_conversion!(i8; i8, u8, i16, u16, i32, u32, f32, f64);
impl ConvertElement<ClampedU8> for i8 {
    fn convert_element(self) -> ClampedU8 {
        ClampedU8(self.max(0) as u8)
    }
}
float_conversion!(f32, f64);
