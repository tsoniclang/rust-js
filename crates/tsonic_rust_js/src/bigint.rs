use crate::errors::{range_error, syntax_error, JsResult};
use num_traits::FromPrimitive;
use tsonic_rust_runtime::BigInt;

pub fn from_integer<T: Into<num_bigint::BigInt>>(value: T) -> BigInt {
    BigInt::from(value.into())
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

pub fn as_int_n<T: BigIntBitInput>(bits: f64, value: &T) -> JsResult<BigInt> {
    value.truncate_bits(index_width(bits)?, true)
}

pub fn as_uint_n<T: BigIntBitInput>(bits: f64, value: &T) -> JsResult<BigInt> {
    value.truncate_bits(index_width(bits)?, false)
}

pub trait BigIntBitInput: bit_input::Sealed {
    fn truncate_bits(&self, width: u64, signed: bool) -> JsResult<BigInt>;
}

mod bit_input {
    pub trait Sealed {}
}

impl bit_input::Sealed for BigInt {}

impl BigIntBitInput for BigInt {
    fn truncate_bits(&self, width: u64, signed: bool) -> JsResult<BigInt> {
        if width == 0 { return Ok(from_integer(0_u8)); }
        let bytes = self.to_signed_bytes_le();
        let negative = bytes.last().is_some_and(|byte| byte & 0x80 != 0);
        if (signed || !negative) && width >= bytes.len() as u64 * 8 {
            return Ok(self.clone());
        }
        wrap_signed_bytes(width, bytes, signed)
    }
}

macro_rules! native_bit_inputs {
    ($($native:ty),+ $(,)?) => {$(
        impl bit_input::Sealed for $native {}
        impl BigIntBitInput for $native {
            fn truncate_bits(&self, width: u64, signed: bool) -> JsResult<BigInt> {
                if width == 0 { return Ok(from_integer(0_u8)); }
                let raw = *self as u128;
                if width <= 128 {
                    let mask = if width == 128 { u128::MAX } else { (1_u128 << width) - 1 };
                    let truncated = raw & mask;
                    return Ok(if signed && truncated & (1_u128 << (width - 1)) != 0 {
                        from_integer((truncated | !mask) as i128)
                    } else {
                        from_integer(truncated)
                    });
                }
                let negative = <$native>::MIN != 0 && (*self as i128) < 0;
                if signed || !negative {
                    return Ok(from_integer(*self));
                }
                wrap_signed_bytes(width, raw.to_le_bytes().to_vec(), signed)
            }
        }
    )+};
}

native_bit_inputs!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

pub fn to_string_radix(value: &BigInt, radix: f64) -> JsResult<String> {
    let radix = crate::coercion::to_integer_or_infinity(radix);
    if !(2.0..=36.0).contains(&radix) {
        return Err(range_error("BigInt radix must be between 2 and 36"));
    }
    Ok(value.to_str_radix(radix as u32))
}

fn index_width(bits: f64) -> JsResult<u64> {
    let width = crate::coercion::to_integer_or_infinity(bits);
    if !(0.0..=crate::number::MAX_SAFE_INTEGER).contains(&width) {
        return Err(range_error("BigInt bit width is outside the index range"));
    }
    Ok(width as u64)
}

fn wrap_signed_bytes(width: u64, mut bytes: Vec<u8>, signed: bool) -> JsResult<BigInt> {
    let negative = bytes.last().is_some_and(|byte| byte & 0x80 != 0);
    let length = usize::try_from(width.div_ceil(8))
        .map_err(|_| range_error("BigInt result exceeds addressable storage"))?;
    let capacity = length
        .checked_add(1)
        .ok_or_else(|| range_error("BigInt result exceeds addressable storage"))?;
    bytes
        .try_reserve_exact(capacity.saturating_sub(bytes.len()))
        .map_err(|_| range_error("BigInt result cannot be allocated"))?;
    bytes.resize(length, if negative { 0xff } else { 0 });
    let high_bits = ((width - 1) % 8 + 1) as u32;
    let mask = ((1_u16 << high_bits) - 1) as u8;
    let last = bytes.last_mut().expect("a nonzero bit width has storage");
    *last &= mask;
    if signed && *last & (1_u8 << (high_bits - 1)) != 0 {
        *last |= !mask;
    } else if *last & 0x80 != 0 {
        bytes.push(0);
    }
    Ok(BigInt::from_signed_bytes_le(&bytes))
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
