pub(crate) fn native_index(value: f64) -> isize {
    value as isize
}

pub(crate) fn relative_index(value: f64, length: usize) -> Option<usize> {
    let index = native_index(value);
    let position = if index < 0 {
        length.checked_sub(index.unsigned_abs())?
    } else {
        index as usize
    };
    (position < length).then_some(position)
}

pub(crate) fn absolute_index(value: f64, length: usize) -> Option<usize> {
    let position = usize::try_from(native_index(value)).ok()?;
    (position < length).then_some(position)
}

pub(crate) fn native_length(value: f64) -> crate::errors::JsResult<usize> {
    const EXCLUSIVE_MAXIMUM: f64 = (usize::MAX as u128 + 1) as f64;
    if value.fract() != 0.0 || !(0.0..EXCLUSIVE_MAXIMUM).contains(&value) {
        return Err(crate::errors::range_error("length must be a non-negative native integer"));
    }
    Ok(value as usize)
}

pub(crate) fn normalize_slice_index(value: f64, length: usize) -> usize {
    let index = native_index(value);
    if index < 0 {
        length.saturating_sub(index.unsigned_abs())
    } else {
        (index as usize).min(length)
    }
}

#[cfg(test)]
mod tests {
    use super::{absolute_index, native_index, normalize_slice_index, relative_index};

    #[test]
    fn indices_follow_native_casts_and_integer_bounds() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, 1.9, -1.9,
            9_007_199_254_740_992.0, isize::MIN as f64, isize::MAX as f64] {
            assert_eq!(native_index(value), value as isize);
        }
        assert_eq!(relative_index(-1.0, 3), Some(2));
        assert_eq!(relative_index(-4.0, 3), None);
        assert_eq!(absolute_index(-1.0, 3), None);
        assert_eq!(normalize_slice_index(-2.0, 3), 1);
        assert_eq!(normalize_slice_index(f64::NEG_INFINITY, 3), 0);
        assert_eq!(normalize_slice_index(f64::INFINITY, 3), 3);
        if usize::BITS == 64 {
            let length = usize::try_from(9_007_199_254_740_993_u64).unwrap();
            assert_eq!(relative_index(-1.0, length), Some(length - 1));
            assert_eq!(normalize_slice_index(-1.0, length), length - 1);
        }
    }
}
