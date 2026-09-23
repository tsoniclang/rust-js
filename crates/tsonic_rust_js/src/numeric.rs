use tsonic_rust_runtime::conversions::IntegerInput;

pub trait IndexInput: IntegerInput<usize> {
    fn property_index(self) -> Option<usize>;
    fn length_index(self) -> Option<usize>;
    fn relative_index(self, length: usize) -> Option<usize>;
    fn clamped_index(self, length: usize) -> usize;
    fn floor_index(self) -> Option<usize>;
    fn positive_index(self, length: usize) -> usize;
    fn last_index(self, length: usize) -> Option<usize>;
    fn valid_repeat_count(self) -> bool;
}

macro_rules! unsigned_indices {
    ($($input:ty),+ $(,)?) => {$(
        impl IndexInput for $input {
            #[inline]
            fn property_index(self) -> Option<usize> {
                Some(usize::try_from(self).unwrap_or(usize::MAX))
            }

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

            #[inline]
            fn positive_index(self, length: usize) -> usize { self.clamped_index(length) }

            #[inline]
            fn last_index(self, length: usize) -> Option<usize> {
                Some(self.positive_index(length.checked_sub(1)?))
            }

            #[inline]
            fn valid_repeat_count(self) -> bool { true }
        }
    )+};
}

macro_rules! signed_indices {
    ($($input:ty),+ $(,)?) => {$(
        impl IndexInput for $input {
            #[inline]
            fn property_index(self) -> Option<usize> {
                (self >= 0).then(|| usize::try_from(self).unwrap_or(usize::MAX))
            }

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

            #[inline]
            fn positive_index(self, length: usize) -> usize {
                if self < 0 { 0 } else { self.clamped_index(length) }
            }

            #[inline]
            fn last_index(self, length: usize) -> Option<usize> {
                if self < 0 { self.relative_index(length) }
                else { Some(self.positive_index(length.checked_sub(1)?)) }
            }

            #[inline]
            fn valid_repeat_count(self) -> bool { self >= 0 }
        }
    )+};
}

unsigned_indices!(u8, u16, u32, u64, u128, usize);
signed_indices!(i8, i16, i32, i64, i128, isize);

impl IndexInput for f64 {
    #[inline]
    fn property_index(self) -> Option<usize> {
        (self.is_finite() && self >= 0.0 && self.trunc() == self).then_some(self as usize)
    }

    #[inline]
    fn positive_index(self, length: usize) -> usize {
        (self as usize).min(length)
    }

    #[inline]
    fn last_index(self, length: usize) -> Option<usize> {
        if self.trunc() < 0.0 {
            self.relative_index(length)
        } else {
            Some(self.positive_index(length.checked_sub(1)?))
        }
    }

    #[inline]
    fn valid_repeat_count(self) -> bool {
        self.is_nan() || self.is_finite() && self.trunc() >= 0.0
    }
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
    fn property_index(self) -> Option<usize> {
        f64::from(self).property_index()
    }

    #[inline]
    fn positive_index(self, length: usize) -> usize {
        f64::from(self).positive_index(length)
    }

    #[inline]
    fn last_index(self, length: usize) -> Option<usize> {
        f64::from(self).last_index(length)
    }

    #[inline]
    fn valid_repeat_count(self) -> bool {
        f64::from(self).valid_repeat_count()
    }
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
}
