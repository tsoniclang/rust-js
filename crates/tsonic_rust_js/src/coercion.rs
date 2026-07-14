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

fn to_integer_or_infinity(value: f64) -> f64 {
    if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value.trunc()
    }
}

#[cfg(test)]
mod tests {
    use super::relative_index;

    #[test]
    fn relative_indexes_follow_to_integer_or_infinity() {
        assert_eq!(relative_index(f64::NAN, 3), Some(0));
        assert_eq!(relative_index(1.9, 3), Some(1));
        assert_eq!(relative_index(-1.9, 3), Some(2));
        assert_eq!(relative_index(f64::INFINITY, 3), None);
        assert_eq!(relative_index(f64::NEG_INFINITY, 3), None);
        assert_eq!(relative_index(-4.0, 3), None);
    }
}
