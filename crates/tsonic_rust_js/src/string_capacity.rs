use crate::errors::{range_error, JsResult};
use crate::native_integer::{integer_or_infinity, native_length};

pub(crate) fn repeat_shape(count: f64, length: impl FnOnce() -> usize) -> JsResult<(usize, usize)> {
    let count = integer_or_infinity(count);
    if count < 0.0 || !count.is_finite() {
        return Err(range_error("invalid repeat count"));
    }
    if count == 0.0 {
        return Ok((0, 0));
    }
    let length = length();
    if length == 0 {
        return Ok((0, 0));
    }
    let count = native_length(count)?;
    let length = length
        .checked_mul(count)
        .ok_or_else(|| range_error("invalid string length"))?;
    Ok((count, length))
}

#[cfg(test)]
mod tests {
    use super::repeat_shape;
    use tsonic_rust_runtime::JsErrorKind;

    #[test]
    fn zero_repeat_counts_do_not_measure_length() {
        for count in [0.0, -0.0, f64::NAN, -0.9, 0.9] {
            let shape = repeat_shape(count, || panic!("zero repeat measured input length"));
            assert_eq!(shape.unwrap(), (0, 0));
        }
    }

    #[test]
    fn invalid_repeat_counts_do_not_measure_length() {
        for count in [-1.0, -1.9, f64::NEG_INFINITY, f64::INFINITY] {
            let error = repeat_shape(count, || panic!("invalid repeat measured input length"));
            assert_eq!(error.unwrap_err().kind(), JsErrorKind::RangeError);
        }
    }

    #[test]
    fn nonzero_repeat_counts_measure_length_once() {
        for (input_length, count, expected) in [(0, f64::MAX, (0, 0)), (3, 3.9, (3, 9))] {
            let mut measurements = 0;
            let shape = repeat_shape(count, || {
                measurements += 1;
                input_length
            });
            assert_eq!(shape.unwrap(), expected);
            assert_eq!(measurements, 1);
        }
    }
}
