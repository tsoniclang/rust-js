//! Number helpers used by generated code for JavaScript-compatible semantics.

use std::cmp::Ordering;
use std::str::FromStr;

use num_bigint::{BigInt, BigUint};
use num_traits::{One, ToPrimitive, Zero};
use tsonic_rust_runtime::{JsError, JsErrorKind};

pub const MAX_VALUE: f64 = f64::MAX;
pub const MIN_VALUE: f64 = f64::from_bits(1);
pub const EPSILON: f64 = f64::EPSILON;
pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
pub const MIN_SAFE_INTEGER: f64 = -9_007_199_254_740_991.0;
pub const POSITIVE_INFINITY: f64 = f64::INFINITY;
pub const NEGATIVE_INFINITY: f64 = f64::NEG_INFINITY;
pub const NAN: f64 = f64::NAN;

pub trait JsNumberValue: Copy {
    fn to_js_f64(self) -> f64;
    fn to_js_decimal_string(self) -> String;
}

pub trait JsIntegerValue: JsNumberValue {
    fn to_js_radix_string(self, radix: u32) -> String;
}

macro_rules! impl_signed_integer {
    ($($type:ty),+ $(,)?) => {
        $(
            impl JsNumberValue for $type {
                fn to_js_f64(self) -> f64 {
                    self as f64
                }

                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }
            }

            impl JsIntegerValue for $type {
                fn to_js_radix_string(self, radix: u32) -> String {
                    BigInt::from(self).to_str_radix(radix)
                }
            }
        )+
    };
}

macro_rules! impl_unsigned_integer {
    ($($type:ty),+ $(,)?) => {
        $(
            impl JsNumberValue for $type {
                fn to_js_f64(self) -> f64 {
                    self as f64
                }

                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }
            }

            impl JsIntegerValue for $type {
                fn to_js_radix_string(self, radix: u32) -> String {
                    BigUint::from(self).to_str_radix(radix)
                }
            }
        )+
    };
}

impl_signed_integer!(i8, i16, i32, i64, isize);
impl_unsigned_integer!(u8, u16, u32, u64, usize);

impl JsNumberValue for f32 {
    fn to_js_f64(self) -> f64 {
        f64::from(self)
    }

    fn to_js_decimal_string(self) -> String {
        format_number(f64::from(self))
    }
}

impl JsNumberValue for f64 {
    fn to_js_f64(self) -> f64 {
        self
    }

    fn to_js_decimal_string(self) -> String {
        format_number(self)
    }
}

pub fn to_string<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn value_of<T: JsNumberValue>(value: T) -> T {
    value
}

pub fn to_string_radix<T: JsIntegerValue>(value: T, radix: f64) -> Result<String, JsError> {
    let radix = integer_parameter(radix, 2, 36, "toString radix")?;
    Ok(value.to_js_radix_string(u32::from(radix)))
}

pub fn parse_int(text: &str, radix: Option<f64>) -> f64 {
    let mut source = trim_ecmascript_start(text);
    if source.is_empty() {
        return f64::NAN;
    }

    let negative = match source.as_bytes().first() {
        Some(b'+') => {
            source = &source[1..];
            false
        }
        Some(b'-') => {
            source = &source[1..];
            true
        }
        _ => false,
    };

    let mut base = radix.map_or(0, to_int32);
    if base != 0 && !(2..=36).contains(&base) {
        return f64::NAN;
    }

    let strip_prefix = base == 0 || base == 16;
    if base == 0 {
        base = 10;
    }
    if strip_prefix
        && (source.as_bytes().starts_with(b"0x") || source.as_bytes().starts_with(b"0X"))
    {
        base = 16;
        source = &source[2..];
    }

    let radix = u32::try_from(base).expect("validated parseInt radix");
    let mut value = BigUint::zero();
    let mut consumed = false;
    for byte in source.bytes() {
        let Some(digit) = ascii_digit(byte) else {
            break;
        };
        if digit >= radix {
            break;
        }
        value = value * radix + digit;
        consumed = true;
    }

    if !consumed {
        return f64::NAN;
    }

    let magnitude = value.to_f64().expect("BigUint always converts to f64");
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

pub fn parse_int_default(text: &str) -> f64 {
    parse_int(text, None)
}

pub fn parse_int_radix(text: &str, radix: f64) -> f64 {
    parse_int(text, Some(radix))
}

pub fn parse_float(text: &str) -> f64 {
    let source = trim_ecmascript_start(text);
    if source.is_empty() {
        return f64::NAN;
    }

    let bytes = source.as_bytes();
    let mut cursor = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    if source[cursor..].starts_with("Infinity") {
        return if bytes.first() == Some(&b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let integer_start = cursor;
    while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
        cursor += 1;
    }
    let mut digit_count = cursor - integer_start;
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let fraction_start = cursor;
        while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
            cursor += 1;
        }
        digit_count += cursor - fraction_start;
    }
    if digit_count == 0 {
        return f64::NAN;
    }

    let exponent_start = cursor;
    if matches!(bytes.get(cursor), Some(b'e') | Some(b'E')) {
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+') | Some(b'-')) {
            cursor += 1;
        }
        let exponent_digits = cursor;
        while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
            cursor += 1;
        }
        if cursor == exponent_digits {
            cursor = exponent_start;
        }
    }

    f64::from_str(&source[..cursor]).unwrap_or_else(|_| {
        if bytes.first() == Some(&b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

pub fn is_nan(value: f64) -> bool {
    value.is_nan()
}

pub fn is_finite(value: f64) -> bool {
    value.is_finite()
}

pub fn is_integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

pub fn is_safe_integer(value: f64) -> bool {
    is_integer(value) && (MIN_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value)
}

pub fn to_fixed<T: JsNumberValue>(value: T, digits: Option<f64>) -> Result<String, JsError> {
    let digits = integer_parameter(digits.unwrap_or(0.0), 0, 100, "toFixed digits")?;
    Ok(ryu_js::Buffer::new()
        .format_to_fixed(value.to_js_f64(), digits)
        .to_owned())
}

pub fn to_fixed_default<T: JsNumberValue>(value: T) -> String {
    ryu_js::Buffer::new()
        .format_to_fixed(value.to_js_f64(), 0)
        .to_owned()
}

pub fn to_fixed_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_fixed(value, Some(digits))
}

pub fn to_exponential<T: JsNumberValue>(value: T, digits: Option<f64>) -> Result<String, JsError> {
    let value = value.to_js_f64();
    let Some(digits) = digits else {
        return Ok(shortest_exponential(value));
    };
    let fraction_digits = integer_parameter(digits, 0, 100, "toExponential digits")?;
    Ok(fixed_significant_string(
        value,
        usize::from(fraction_digits) + 1,
        SignificantFormat::Exponential,
    ))
}

pub fn to_exponential_default<T: JsNumberValue>(value: T) -> String {
    shortest_exponential(value.to_js_f64())
}

pub fn to_exponential_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_exponential(value, Some(digits))
}

pub fn to_precision<T: JsNumberValue>(value: T, precision: Option<f64>) -> Result<String, JsError> {
    let Some(precision) = precision else {
        return Ok(value.to_js_decimal_string());
    };
    let precision = integer_parameter(precision, 1, 100, "toPrecision precision")?;
    Ok(fixed_significant_string(
        value.to_js_f64(),
        usize::from(precision),
        SignificantFormat::Precision,
    ))
}

pub fn to_precision_default<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn to_precision_digits<T: JsNumberValue>(value: T, precision: f64) -> Result<String, JsError> {
    to_precision(value, Some(precision))
}

fn format_number(value: f64) -> String {
    ryu_js::Buffer::new().format(value).to_owned()
}

fn integer_parameter(value: f64, minimum: u8, maximum: u8, name: &str) -> Result<u8, JsError> {
    let integer = if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value.trunc()
    };
    if !integer.is_finite() || integer < f64::from(minimum) || integer > f64::from(maximum) {
        return Err(JsError::new(
            JsErrorKind::RangeError,
            format!("{name} must be between {minimum} and {maximum}"),
        ));
    }
    Ok(integer as u8)
}

fn to_int32(value: f64) -> i32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let modulo = value.trunc().rem_euclid(4_294_967_296.0);
    if modulo >= 2_147_483_648.0 {
        (modulo - 4_294_967_296.0) as i32
    } else {
        modulo as i32
    }
}

fn ascii_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'Z' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn trim_ecmascript_start(value: &str) -> &str {
    value.trim_start_matches(is_ecmascript_whitespace)
}

fn is_ecmascript_whitespace(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

enum SignificantFormat {
    Exponential,
    Precision,
}

fn fixed_significant_string(value: f64, precision: usize, format: SignificantFormat) -> String {
    if !value.is_finite() {
        return format_number(value);
    }
    let negative = value.is_sign_negative() && value != 0.0;
    let (digits, exponent) = rounded_significand(value.abs(), precision);
    let unsigned = match format {
        SignificantFormat::Exponential => scientific_string(&digits, exponent),
        SignificantFormat::Precision if exponent < -6 || exponent >= precision as i32 => {
            scientific_string(&digits, exponent)
        }
        SignificantFormat::Precision => fixed_precision_string(&digits, exponent),
    };
    if negative {
        format!("-{unsigned}")
    } else {
        unsigned
    }
}

fn rounded_significand(value: f64, precision: usize) -> (String, i32) {
    if value == 0.0 {
        return ("0".repeat(precision), 0);
    }
    let (numerator, denominator) = exact_positive_rational(value);
    let mut exponent = decimal_exponent(&numerator, &denominator, value);
    let scale = precision as i32 - 1 - exponent;
    let (scaled_numerator, scaled_denominator) = if scale >= 0 {
        (numerator * power_of_ten(scale as usize), denominator)
    } else {
        (numerator, denominator * power_of_ten((-scale) as usize))
    };
    let mut rounded = &scaled_numerator / &scaled_denominator;
    let remainder = scaled_numerator % &scaled_denominator;
    if remainder << 1 >= scaled_denominator {
        rounded += BigUint::one();
    }
    let limit = power_of_ten(precision);
    if rounded >= limit {
        rounded /= 10_u8;
        exponent += 1;
    }
    let digits = rounded.to_str_radix(10);
    (
        format!("{}{}", "0".repeat(precision - digits.len()), digits),
        exponent,
    )
}

fn exact_positive_rational(value: f64) -> (BigUint, BigUint) {
    let bits = value.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent_bits - 1023 - 52)
    };
    if exponent >= 0 {
        (
            BigUint::from(significand) << exponent as usize,
            BigUint::one(),
        )
    } else {
        (
            BigUint::from(significand),
            BigUint::one() << (-exponent) as usize,
        )
    }
}

fn decimal_exponent(numerator: &BigUint, denominator: &BigUint, value: f64) -> i32 {
    let mut exponent = value.log10().floor() as i32;
    while compare_to_power_of_ten(numerator, denominator, exponent) == Ordering::Less {
        exponent -= 1;
    }
    while compare_to_power_of_ten(numerator, denominator, exponent + 1) != Ordering::Less {
        exponent += 1;
    }
    exponent
}

fn compare_to_power_of_ten(numerator: &BigUint, denominator: &BigUint, exponent: i32) -> Ordering {
    if exponent >= 0 {
        numerator.cmp(&(denominator * power_of_ten(exponent as usize)))
    } else {
        (numerator * power_of_ten((-exponent) as usize)).cmp(denominator)
    }
}

fn power_of_ten(exponent: usize) -> BigUint {
    BigUint::from(10_u8).pow(u32::try_from(exponent).expect("bounded decimal exponent"))
}

fn scientific_string(digits: &str, exponent: i32) -> String {
    let mut result = digits[..1].to_owned();
    if digits.len() > 1 {
        result.push('.');
        result.push_str(&digits[1..]);
    }
    result.push('e');
    if exponent >= 0 {
        result.push('+');
    }
    result.push_str(&exponent.to_string());
    result
}

fn fixed_precision_string(digits: &str, exponent: i32) -> String {
    if exponent < 0 {
        format!("0.{}{}", "0".repeat((-exponent - 1) as usize), digits)
    } else {
        let point = exponent as usize + 1;
        if point >= digits.len() {
            format!("{}{}", digits, "0".repeat(point - digits.len()))
        } else {
            format!("{}.{}", &digits[..point], &digits[point..])
        }
    }
}

fn shortest_exponential(value: f64) -> String {
    if !value.is_finite() {
        return format_number(value);
    }
    if value == 0.0 {
        return "0e+0".to_owned();
    }
    let negative = value.is_sign_negative();
    let source = format_number(value.abs());
    let (mut digits, exponent) = if let Some((mantissa, exponent)) = source.split_once('e') {
        (
            mantissa.replace('.', ""),
            exponent
                .trim_start_matches('+')
                .parse::<i32>()
                .expect("Ryū exponent"),
        )
    } else {
        let decimal_position = source.find('.').unwrap_or(source.len());
        let raw = source.replace('.', "");
        let leading_zeroes = raw.bytes().take_while(|byte| *byte == b'0').count();
        (
            raw[leading_zeroes..].to_owned(),
            decimal_position as i32 - leading_zeroes as i32 - 1,
        )
    };
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    let result = scientific_string(&digits, exponent);
    if negative {
        format!("-{result}")
    } else {
        result
    }
}
