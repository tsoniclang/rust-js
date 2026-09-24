pub(crate) fn integer_or_infinity(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.trunc()
    }
}

pub(crate) fn index_length(
    value: impl crate::numeric::IndexInput,
) -> crate::errors::JsResult<usize> {
    value
        .length_index()
        .ok_or_else(|| crate::errors::range_error("length must be a non-negative native integer"))
}

pub(crate) fn pad_length(value: impl crate::numeric::IndexInput) -> usize {
    value.positive_index(usize::MAX)
}

pub trait Integer32 {
    fn integer32(self) -> u32;
}

macro_rules! native_integer32 {
    ($($native:ty),+ $(,)?) => {$(
        impl Integer32 for $native {
            #[inline]
            fn integer32(self) -> u32 { self as u32 }
        }
    )+};
}

native_integer32!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

impl Integer32 for f32 {
    #[inline]
    fn integer32(self) -> u32 {
        f64::from(self).integer32()
    }
}

impl Integer32 for f64 {
    #[inline]
    fn integer32(self) -> u32 {
        let bits = self.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
        if !(0..84).contains(&exponent) {
            return 0;
        }
        let significand = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
        let magnitude = if exponent < 52 {
            (significand >> (52 - exponent)) as u32
        } else {
            (significand as u32).wrapping_shl((exponent - 52) as u32)
        };
        if bits >> 63 != 0 {
            magnitude.wrapping_neg()
        } else {
            magnitude
        }
    }
}

pub(crate) fn split_limit(value: Option<impl Integer32>) -> usize {
    value.map_or(usize::MAX, |value| value.integer32() as usize)
}

pub(crate) fn relative_index(
    value: impl crate::numeric::IndexInput,
    length: usize,
) -> Option<usize> {
    value.relative_index(length)
}

pub(crate) fn absolute_index(
    value: impl crate::numeric::IndexInput,
    length: usize,
) -> Option<usize> {
    let position = value.length_index()?;
    (position < length).then_some(position)
}

pub(crate) fn native_length(
    value: impl crate::numeric::IndexInput,
) -> crate::errors::JsResult<usize> {
    value
        .checked_integer()
        .ok_or_else(|| crate::errors::range_error("length must be a non-negative native integer"))
}

pub(crate) fn normalize_slice_index(
    value: impl crate::numeric::IndexInput,
    length: usize,
) -> usize {
    value.clamped_index(length)
}

#[cfg(test)]
mod tests {
    use super::{absolute_index, normalize_slice_index, relative_index};

    #[test]
    fn indices_follow_native_casts_and_integer_bounds() {
        assert_eq!(relative_index(-1.0, 3), Some(2));
        assert_eq!(relative_index(-4.0, 3), None);
        assert_eq!(relative_index(f64::NAN, 3), Some(0));
        assert_eq!(relative_index(1.9, 3), Some(1));
        assert_eq!(relative_index(-1.9, 3), Some(2));
        assert_eq!(relative_index(f64::INFINITY, usize::MAX), None);
        assert_eq!(relative_index(f64::NEG_INFINITY, usize::MAX), None);
        assert_eq!(relative_index(f64::MIN, usize::MAX), None);
        assert_eq!(absolute_index(-1.0, 3), None);
        assert_eq!(absolute_index(f64::NAN, 3), Some(0));
        assert_eq!(absolute_index(1.9, 3), Some(1));
        assert_eq!(absolute_index(f64::INFINITY, usize::MAX), None);
        assert_eq!(normalize_slice_index(f64::NAN, 3), 0);
        assert_eq!(normalize_slice_index(1.9, 3), 1);
        assert_eq!(normalize_slice_index(-1.9, 3), 2);
        assert_eq!(normalize_slice_index(-2.0, 3), 1);
        assert_eq!(normalize_slice_index(f64::NEG_INFINITY, 3), 0);
        assert_eq!(normalize_slice_index(f64::INFINITY, 3), 3);
        assert_eq!(normalize_slice_index(f64::INFINITY, usize::MAX), usize::MAX);
        let large_index = (isize::MAX as usize) + 1;
        assert_eq!(
            absolute_index(large_index as f64, usize::MAX),
            Some(large_index)
        );
        assert_eq!(
            relative_index(large_index as f64, usize::MAX),
            Some(large_index)
        );
        if usize::BITS == 64 {
            let length = usize::try_from(9_007_199_254_740_993_u64).unwrap();
            assert_eq!(relative_index(-1.0, length), Some(length - 1));
            assert_eq!(normalize_slice_index(-1.0, length), length - 1);
        }
    }
}
