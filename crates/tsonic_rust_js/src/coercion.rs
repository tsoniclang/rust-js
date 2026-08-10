pub(crate) fn relative_index(value: f64, length: usize) -> Option<usize> {
    let integer = to_integer_or_infinity(value);
    if !integer.is_finite() {
        return None;
    }
    let length = length as f64;
    let index = if integer < 0.0 {
        length + integer
    } else {
        integer
    };
    if index < 0.0 || index >= length {
        return None;
    }
    Some(index as usize)
}

pub(crate) fn absolute_index(value: f64, length: usize) -> Option<usize> {
    let integer = to_integer_or_infinity(value);
    if !integer.is_finite() || integer < 0.0 || integer >= length as f64 {
        return None;
    }
    Some(integer as usize)
}

pub(crate) fn to_integer_or_infinity(value: f64) -> f64 {
    if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value.trunc()
    }
}

pub(crate) fn normalize_slice_index(value: f64, length: usize) -> usize {
    let integer = to_integer_or_infinity(value);
    if integer == f64::NEG_INFINITY {
        return 0;
    }
    if integer == f64::INFINITY {
        return length;
    }
    if integer < 0.0 {
        return (length as f64 + integer).max(0.0) as usize;
    }
    integer.min(length as f64) as usize
}

#[cfg(test)]
mod tests {
    use super::{absolute_index, normalize_slice_index, relative_index};

    #[test]
    fn relative_indexes_follow_to_integer_or_infinity() {
        assert_eq!(relative_index(f64::NAN, 3), Some(0));
        assert_eq!(relative_index(1.9, 3), Some(1));
        assert_eq!(relative_index(-1.9, 3), Some(2));
        assert_eq!(relative_index(f64::INFINITY, 3), None);
        assert_eq!(relative_index(f64::NEG_INFINITY, 3), None);
        assert_eq!(relative_index(-4.0, 3), None);
    }

    #[test]
    fn slice_indexes_follow_to_integer_or_infinity_and_clamp() {
        assert_eq!(normalize_slice_index(f64::NAN, 3), 0);
        assert_eq!(normalize_slice_index(1.9, 3), 1);
        assert_eq!(normalize_slice_index(-1.9, 3), 2);
        assert_eq!(normalize_slice_index(f64::INFINITY, 3), 3);
        assert_eq!(normalize_slice_index(f64::NEG_INFINITY, 3), 0);
        assert_eq!(normalize_slice_index(99.0, 3), 3);
    }

    #[test]
    fn absolute_indexes_never_treat_negative_values_as_relative() {
        assert_eq!(absolute_index(f64::NAN, 3), Some(0));
        assert_eq!(absolute_index(1.9, 3), Some(1));
        assert_eq!(absolute_index(-1.0, 3), None);
        assert_eq!(absolute_index(f64::INFINITY, 3), None);
    }
}
