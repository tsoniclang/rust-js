pub trait IntegerInput<Output>: Copy {
    fn checked_integer(self) -> Option<Output>;
    fn truncated_integer(self) -> Option<Output>;
}

pub trait IndexInput: IntegerInput<usize> {
    fn length_index(self) -> Option<usize>;
    fn relative_index(self, length: usize) -> Option<usize>;
    fn clamped_index(self, length: usize) -> usize;
    fn floor_index(self) -> Option<usize>;
}

macro_rules! unsigned_indices {
    ($($input:ty),+ $(,)?) => {$(
        impl IndexInput for $input {
            #[inline]
            fn length_index(self) -> Option<usize> { self.checked_integer() }

            #[inline]
            fn relative_index(self, length: usize) -> Option<usize> {
                self.length_index().filter(|index| *index < length)
            }

            #[inline]
            fn clamped_index(self, length: usize) -> usize {
                self.length_index().unwrap_or(usize::MAX).min(length)
            }

            #[inline]
            fn floor_index(self) -> Option<usize> { self.checked_integer() }
        }
    )+};
}

macro_rules! signed_indices {
    ($($input:ty),+ $(,)?) => {$(
        impl IndexInput for $input {
            #[inline]
            fn length_index(self) -> Option<usize> { self.checked_integer() }

            #[inline]
            fn relative_index(self, length: usize) -> Option<usize> {
                let index = if self < 0 {
                    length.checked_sub(usize::try_from(self.unsigned_abs()).ok()?)?
                } else { self.length_index()? };
                (index < length).then_some(index)
            }

            #[inline]
            fn clamped_index(self, length: usize) -> usize {
                if self < 0 {
                    length.saturating_sub(usize::try_from(self.unsigned_abs()).unwrap_or(usize::MAX))
                } else { self.length_index().unwrap_or(usize::MAX).min(length) }
            }

            #[inline]
            fn floor_index(self) -> Option<usize> { self.checked_integer() }
        }
    )+};
}

unsigned_indices!(u8, u16, u32, u64, u128, usize);
signed_indices!(i8, i16, i32, i64, i128, isize);

impl IndexInput for f64 {
    #[inline]
    fn length_index(self) -> Option<usize> {
        if self.is_nan() {
            Some(0)
        } else {
            self.trunc().checked_integer()
        }
    }

    #[inline]
    fn relative_index(self, length: usize) -> Option<usize> {
        let value = self.trunc();
        let index = if value < 0.0 {
            length.checked_sub((-value).length_index()?)?
        } else {
            value.length_index()?
        };
        (index < length).then_some(index)
    }

    #[inline]
    fn clamped_index(self, length: usize) -> usize {
        let value = self.trunc();
        if value < 0.0 {
            length.saturating_sub((-value) as usize)
        } else {
            (value as usize).min(length)
        }
    }

    #[inline]
    fn floor_index(self) -> Option<usize> {
        if self.is_finite() {
            self.floor().checked_integer()
        } else {
            Some(0)
        }
    }
}

impl IndexInput for f32 {
    #[inline]
    fn length_index(self) -> Option<usize> {
        f64::from(self).length_index()
    }

    #[inline]
    fn relative_index(self, length: usize) -> Option<usize> {
        f64::from(self).relative_index(length)
    }

    #[inline]
    fn clamped_index(self, length: usize) -> usize {
        f64::from(self).clamped_index(length)
    }

    #[inline]
    fn floor_index(self) -> Option<usize> {
        f64::from(self).floor_index()
    }
}

macro_rules! integral_inputs {
    ($($input:ty),+ $(,)?) => {$(
        impl<Output: TryFrom<$input>> IntegerInput<Output> for $input {
            #[inline]
            fn checked_integer(self) -> Option<Output> {
                Output::try_from(self).ok()
            }

            #[inline]
            fn truncated_integer(self) -> Option<Output> {
                self.checked_integer()
            }
        }
    )+};
}

integral_inputs!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);

macro_rules! floating_inputs {
    ($($output:ty),+ $(,)?) => {$(
        impl IntegerInput<$output> for f64 {
            #[inline]
            fn checked_integer(self) -> Option<$output> {
                if self.fract() != 0.0 { return None; }
                self.truncated_integer()
            }

            #[inline]
            fn truncated_integer(self) -> Option<$output> {
                if self.is_nan() { return Some(0); }
                let upper = <$output>::MAX as f64;
                let too_large = if <$output>::BITS > 53 { self >= upper } else { self > upper };
                if !self.is_finite() || self < <$output>::MIN as f64 || too_large {
                    return None;
                }
                Some(self as $output)
            }
        }

        impl IntegerInput<$output> for f32 {
            #[inline]
            fn checked_integer(self) -> Option<$output> {
                f64::from(self).checked_integer()
            }

            #[inline]
            fn truncated_integer(self) -> Option<$output> {
                f64::from(self).truncated_integer()
            }
        }
    )+};
}

floating_inputs!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);

#[cfg(test)]
mod tests {
    use super::{IndexInput, IntegerInput};

    #[test]
    fn indices_keep_native_integer_widths_and_signed_boundaries() {
        for value in [0_usize, 1, usize::MAX] {
            assert_eq!(value.length_index(), Some(value));
            assert_eq!(value.checked_integer(), Some(value));
            assert_eq!(value.floor_index(), Some(value));
            assert_eq!(value.clamped_index(usize::MAX), value);
        }
        assert_eq!((-1_i64).relative_index(usize::MAX), Some(usize::MAX - 1));
        assert_eq!(i128::MIN.relative_index(usize::MAX), None);
        assert_eq!(i128::MIN.clamped_index(usize::MAX), 0);
        assert_eq!(u128::MAX.clamped_index(4), 4);
        assert_eq!(IntegerInput::<usize>::checked_integer(u128::MAX), None);
        if usize::BITS == 64 {
            let exact = usize::try_from(9_007_199_254_740_993_u64).unwrap();
            assert_eq!(exact.length_index(), Some(exact));
            assert_eq!(exact.relative_index(usize::MAX), Some(exact));
            assert_eq!((-1_i64).relative_index(exact), Some(exact - 1));
        }
    }

    #[test]
    fn floating_indices_retain_each_operations_existing_rules() {
        assert_eq!(f64::NAN.length_index(), Some(0));
        assert_eq!(IntegerInput::<usize>::checked_integer(f64::NAN), None);
        assert_eq!(1.9_f64.length_index(), Some(1));
        assert_eq!(IntegerInput::<usize>::checked_integer(1.9_f64), None);
        assert_eq!((-0.5_f64).length_index(), Some(0));
        assert_eq!((-0.5_f64).floor_index(), None);
        assert_eq!((-0.5_f64).relative_index(4), Some(0));
        assert_eq!((-0.5_f64).clamped_index(4), 0);
        assert_eq!((-1.9_f64).relative_index(4), Some(3));
        assert_eq!(f64::INFINITY.length_index(), None);
        assert_eq!(f64::INFINITY.relative_index(4), None);
        assert_eq!(f64::INFINITY.clamped_index(usize::MAX), usize::MAX);
        assert_eq!(f64::NEG_INFINITY.clamped_index(usize::MAX), 0);
        assert_eq!(f64::INFINITY.floor_index(), Some(0));
        assert_eq!(((usize::MAX as u128 + 1) as f64).length_index(), None);
    }

    #[test]
    fn native_arguments_do_not_round_through_float() {
        assert_eq!(
            IntegerInput::<u64>::checked_integer(9_007_199_254_740_993_u64),
            Some(9_007_199_254_740_993)
        );
        assert_eq!(
            IntegerInput::<u64>::checked_integer(u128::from(u64::MAX)),
            Some(u64::MAX)
        );
        assert_eq!(
            IntegerInput::<u64>::checked_integer(u128::from(u64::MAX) + 1),
            None
        );
        assert_eq!(IntegerInput::<u8>::checked_integer(-1_i64), None);
        assert_eq!(IntegerInput::<i8>::checked_integer(128_u32), None);
        assert_eq!(
            IntegerInput::<u32>::truncated_integer(u64::from(u32::MAX)),
            Some(u32::MAX)
        );
    }

    #[test]
    fn floating_arguments_keep_exact_index_and_truncating_value_rules() {
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.5,
            18_446_744_073_709_551_616.0,
        ] {
            assert_eq!(IntegerInput::<u64>::checked_integer(value), None);
        }
        assert_eq!(IntegerInput::<u8>::truncated_integer(f64::NAN), Some(0));
        assert_eq!(IntegerInput::<u8>::truncated_integer(254.9), Some(254));
        assert_eq!(IntegerInput::<u8>::truncated_integer(255.1), None);
        assert_eq!(IntegerInput::<i8>::truncated_integer(-127.9), Some(-127));
        assert_eq!(
            IntegerInput::<i64>::checked_integer(-9_223_372_036_854_775_808.0),
            Some(i64::MIN)
        );
        assert_eq!(
            IntegerInput::<i64>::checked_integer(9_223_372_036_854_775_808.0),
            None
        );
        assert_eq!(IntegerInput::<usize>::checked_integer(16_f32), Some(16));
        assert_eq!(IntegerInput::<usize>::checked_integer(16.5_f32), None);
    }
}
