use super::JsArray;
use crate::errors::{range_error, JsResult};

pub trait ArrayLength {
    fn array_length(self) -> JsResult<usize>;
}

pub fn construct_length<T: Default>(length: impl ArrayLength) -> JsResult<JsArray<T>> {
    Ok(JsArray::with_length(length.array_length()?))
}

impl ArrayLength for f64 {
    fn array_length(self) -> JsResult<usize> {
        crate::coercion::native_length(self)
    }
}

impl ArrayLength for f32 {
    fn array_length(self) -> JsResult<usize> {
        f64::from(self).array_length()
    }
}

macro_rules! integer_array_lengths {
    ($($integer:ty),+ $(,)?) => {
        $(impl ArrayLength for $integer {
            fn array_length(self) -> JsResult<usize> {
                usize::try_from(self).map_err(|_| range_error("Invalid array length"))
            }
        })+
    };
}

integer_array_lengths!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
