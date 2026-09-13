use crate::errors::{range_error, syntax_error, JsResult};
use num_traits::FromPrimitive;
use tsonic_rust_runtime::BigInt;

pub fn from_integer<T: Into<num_bigint::BigInt>>(value: T) -> BigInt {
    BigInt::from_signed_bytes_le(&value.into().to_signed_bytes_le())
}

pub fn from_number(value: f64) -> JsResult<BigInt> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(range_error(
            "The number cannot be converted to a BigInt because it is not an integer",
        ));
    }
    let integer = num_bigint::BigInt::from_f64(value)
        .ok_or_else(|| range_error("The number cannot be converted to a BigInt"))?;
    Ok(from_integer(integer))
}

pub fn from_boolean(value: bool) -> BigInt {
    from_integer(u8::from(value))
}

pub fn from_string(value: &str) -> JsResult<BigInt> {
    let text = value.trim_matches(crate::globals::is_ecmascript_whitespace);
    if text.is_empty() {
        return Ok(from_integer(0_u8));
    }
    let (digits, radix) = match text.as_bytes() {
        [b'0', b'x' | b'X', ..] => (&text[2..], 16),
        [b'0', b'o' | b'O', ..] => (&text[2..], 8),
        [b'0', b'b' | b'B', ..] => (&text[2..], 2),
        _ => (text, 10),
    };
    let unsigned = if radix == 10 {
        digits.strip_prefix(['+', '-']).unwrap_or(digits)
    } else {
        digits
    };
    if unsigned.is_empty() || !unsigned.chars().all(|character| character.is_digit(radix)) {
        return Err(syntax_error("Cannot convert the string to a BigInt"));
    }
    let parsed = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix)
        .ok_or_else(|| syntax_error("Cannot convert the string to a BigInt"))?;
    Ok(from_integer(parsed))
}
