use super::ascii_digit;
use num_traits::ToPrimitive;

pub(super) fn unsigned_integer(source: &str, radix: u32) -> f64 {
    if radix == 10 {
        return source.parse().unwrap_or(f64::INFINITY);
    }
    if !radix.is_power_of_two() {
        let mut native = 0_u128;
        let mut wide: Option<num_bigint::BigUint> = None;
        for character in source.bytes() {
            let digit = ascii_digit(character).expect("validated digit");
            if let Some(value) = &mut wide {
                *value *= radix;
                *value += digit;
                if value.bits() > 1024 {
                    return f64::INFINITY;
                }
            } else if let Some(value) = native
                .checked_mul(u128::from(radix))
                .and_then(|value| value.checked_add(u128::from(digit)))
            {
                native = value;
            } else {
                wide = Some(num_bigint::BigUint::from(native) * radix + digit);
            }
        }
        return wide.map_or_else(
            || native as f64,
            |value| value.to_f64().expect("BigUint converts to f64"),
        );
    }
    let width = radix.trailing_zeros();
    let mut significant = 0_u64;
    let mut bits = 0_u32;
    let mut sticky = false;
    for character in source.bytes() {
        let digit = ascii_digit(character).expect("validated digit");
        for index in (0..width).rev() {
            let bit = u64::from((digit >> index) & 1);
            if bits == 0 && bit == 0 {
                continue;
            }
            if bits < 54 {
                significant = (significant << 1) | bit;
            } else {
                sticky |= bit != 0;
            }
            bits += 1;
            if bits > 1024 {
                return f64::INFINITY;
            }
        }
    }
    if bits <= 53 {
        return significant as f64;
    }
    let rounded =
        (significant >> 1) + u64::from(significant & 1 != 0 && (sticky || significant & 2 != 0));
    rounded as f64 * 2_f64.powi(bits as i32 - 53)
}

pub fn numeric_string(source: &str) -> f64 {
    let source = source.trim_matches(super::is_ecmascript_whitespace);
    if source.is_empty() {
        return 0.0;
    }
    if matches!(source, "Infinity" | "+Infinity" | "-Infinity") {
        return if source.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let radix = match source.as_bytes() {
        [b'0', b'x' | b'X', ..] => Some(16),
        [b'0', b'o' | b'O', ..] => Some(8),
        [b'0', b'b' | b'B', ..] => Some(2),
        _ => None,
    };
    if let Some(radix) = radix {
        let digits = &source[2..];
        return if !digits.is_empty()
            && digits
                .bytes()
                .all(|value| ascii_digit(value).is_some_and(|value| value < radix))
        {
            unsigned_integer(digits, radix)
        } else {
            f64::NAN
        };
    }
    if source
        .bytes()
        .any(|value| !matches!(value, b'0'..=b'9' | b'.' | b'+' | b'-' | b'e' | b'E'))
    {
        return f64::NAN;
    }
    source.parse().unwrap_or(f64::NAN)
}
