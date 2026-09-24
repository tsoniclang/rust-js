use super::ClampedU8;
use tsonic_rust_runtime::conversions::IntegerInput;

pub trait SearchElement<Target>: Copy {
    fn search_element(self) -> Option<Target>;
}

macro_rules! integer_targets {
    ($($target:ty),+ $(,)?) => {$(
        impl<Source: IntegerInput<$target>> SearchElement<$target> for Source {
            #[inline]
            fn search_element(self) -> Option<$target> {
                self.checked_integer()
            }
        }
    )+};
}

integer_targets!(i8, u8, i16, u16, i32, u32);

impl<Source: IntegerInput<u8>> SearchElement<ClampedU8> for Source {
    #[inline]
    fn search_element(self) -> Option<ClampedU8> {
        self.checked_integer().map(ClampedU8)
    }
}

macro_rules! integer_sources {
    ($($source:ty),+ $(,)?) => {$(
        impl SearchElement<f32> for $source {
            #[inline]
            fn search_element(self) -> Option<f32> {
                let value = self as f32;
                (IntegerInput::<Self>::checked_integer(value) == Some(self)).then_some(value)
            }
        }
        impl SearchElement<f64> for $source {
            #[inline]
            fn search_element(self) -> Option<f64> {
                let value = self as f64;
                (IntegerInput::<Self>::checked_integer(value) == Some(self)).then_some(value)
            }
        }
    )+};
}

integer_sources!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

macro_rules! float_sources {
    ($source:ty; $($target:ty),+ $(,)?) => {$(
        impl SearchElement<$target> for $source {
            #[inline]
            fn search_element(self) -> Option<$target> {
                let value = self as $target;
                (value as Self == self || self.is_nan()).then_some(value)
            }
        }
    )+};
}

float_sources!(f32; f32, f64);
float_sources!(f64; f32, f64);
