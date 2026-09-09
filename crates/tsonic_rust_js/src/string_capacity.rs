use crate::coercion::to_integer_or_infinity;
use crate::errors::{range_error, JsResult};
use crate::number::MAX_SAFE_INTEGER;

pub(crate) fn repeat_shape(length: usize, count: f64) -> JsResult<(usize, usize)> {
    let count = to_integer_or_infinity(count);
    if count < 0.0 || count == f64::INFINITY {
        return Err(range_error("repeat count must be non-negative and finite"));
    }
    if length == 0 || count == 0.0 {
        return Ok((0, 0));
    }
    if count * length as f64 > MAX_SAFE_INTEGER || count >= usize::MAX as f64 {
        return Err(range_error("invalid string length"));
    }
    let count = count as usize;
    let length = length
        .checked_mul(count)
        .ok_or_else(|| range_error("invalid string length"))?;
    Ok((count, length))
}
