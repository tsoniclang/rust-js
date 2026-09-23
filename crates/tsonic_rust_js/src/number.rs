//! Native numeric operations exposed through the source Number API.

mod borrowed;
mod formatting;
mod parsing;
pub use parsing::numeric_string;

use formatting::{integer_significant, SignificantFormat};
mod numeric;
mod source_numeric;
pub use borrowed::NumericRef;
pub use numeric::{bigint_to_number, JsNumeric};
pub use source_numeric::SourceNumeric;

use std::fmt::{Display, LowerExp, Write};
use std::str::FromStr;
use tsonic_rust_runtime::{JsError, JsErrorKind};

pub const MAX_VALUE: f64 = f64::MAX;
pub const MIN_VALUE: f64 = f64::from_bits(1);
pub const EPSILON: f64 = f64::EPSILON;
pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
pub const MIN_SAFE_INTEGER: f64 = -9_007_199_254_740_991.0;
pub const POSITIVE_INFINITY: f64 = f64::INFINITY;
pub const NEGATIVE_INFINITY: f64 = f64::NEG_INFINITY;
pub const NAN: f64 = f64::NAN;

pub trait JsNumberValue: Copy + NativeNumberPredicate + Display + LowerExp {
    fn is_negative_zero(self) -> bool {
        false
    }
    fn to_js_decimal_string(self) -> String;
    fn fixed_string(self, digits: u8) -> String;
    fn exponential_string(self, digits: Option<u8>) -> String;
    fn precision_string(self, precision: u8) -> String;
}

pub trait NativeNumberPredicate {
    fn native_is_integer(self) -> bool;
    fn native_is_safe_integer(self) -> bool;
    fn native_is_finite(self) -> bool;
    fn native_is_nan(self) -> bool;
}

impl NativeNumberPredicate for &tsonic_rust_runtime::BigInt {
    fn native_is_integer(self) -> bool {
        true
    }
    fn native_is_safe_integer(self) -> bool {
        self.as_ref().bits() <= 53
    }
    fn native_is_finite(self) -> bool {
        true
    }
    fn native_is_nan(self) -> bool {
        false
    }
}

pub trait JsIntegerValue: JsNumberValue {
    fn to_js_radix_string(self, radix: u32) -> String;
}

macro_rules! impl_signed_integer {
    ($($type:ty),+ $(,)?) => {
        $(
            impl NativeNumberPredicate for $type {
                fn native_is_integer(self) -> bool { true }
                fn native_is_safe_integer(self) -> bool { (self as i128).unsigned_abs() < (1_u128 << 53) }
                fn native_is_finite(self) -> bool { true }
                fn native_is_nan(self) -> bool { false }
            }

            impl JsNumberValue for $type {
                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }

                fn fixed_string(self, digits: u8) -> String {
                    fixed_integer(self, usize::from(digits))
                }

                fn exponential_string(self, digits: Option<u8>) -> String {
                    integer_significant(self, digits.map(|value| usize::from(value) + 1), SignificantFormat::Exponential)
                }

                fn precision_string(self, precision: u8) -> String {
                    integer_significant(self, Some(usize::from(precision)), SignificantFormat::Precision)
                }
            }

            impl JsIntegerValue for $type {
                fn to_js_radix_string(self, radix: u32) -> String {
                    integer_radix(self.unsigned_abs() as u128, self < 0, radix)
                }
            }
        )+
    };
}

macro_rules! impl_unsigned_integer {
    ($($type:ty),+ $(,)?) => {
        $(
            impl NativeNumberPredicate for $type {
                fn native_is_integer(self) -> bool { true }
                fn native_is_safe_integer(self) -> bool { (self as u128) < (1_u128 << 53) }
                fn native_is_finite(self) -> bool { true }
                fn native_is_nan(self) -> bool { false }
            }

            impl JsNumberValue for $type {
                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }

                fn fixed_string(self, digits: u8) -> String {
                    fixed_integer(self, usize::from(digits))
                }

                fn exponential_string(self, digits: Option<u8>) -> String {
                    integer_significant(self, digits.map(|value| usize::from(value) + 1), SignificantFormat::Exponential)
                }

                fn precision_string(self, precision: u8) -> String {
                    integer_significant(self, Some(usize::from(precision)), SignificantFormat::Precision)
                }
            }

            impl JsIntegerValue for $type {
                fn to_js_radix_string(self, radix: u32) -> String {
                    integer_radix(self as u128, false, radix)
                }
            }
        )+
    };
}

impl_signed_integer!(i8, i16, i32, i64, i128, isize);
impl_unsigned_integer!(u8, u16, u32, u64, u128, usize);

impl NativeNumberPredicate for f32 {
    fn native_is_integer(self) -> bool {
        self.is_finite() && self.fract() == 0.0
    }
    fn native_is_safe_integer(self) -> bool {
        self.native_is_integer() && self.abs() < 9_007_199_254_740_992_f32
    }
    fn native_is_finite(self) -> bool {
        self.is_finite()
    }
    fn native_is_nan(self) -> bool {
        self.is_nan()
    }
}

impl JsNumberValue for f32 {
    fn is_negative_zero(self) -> bool {
        self == 0.0 && self.is_sign_negative()
    }
    fn to_js_decimal_string(self) -> String {
        ryu_js::Buffer::new().format(self).to_owned()
    }

    fn fixed_string(self, digits: u8) -> String {
        formatting::fixed_string(f64::from(self), digits)
    }

    fn exponential_string(self, digits: Option<u8>) -> String {
        match digits {
            None => formatting::shortest_exponential(self),
            Some(digits) => formatting::fixed_significant_string(
                f64::from(self),
                usize::from(digits) + 1,
                SignificantFormat::Exponential,
            ),
        }
    }

    fn precision_string(self, precision: u8) -> String {
        formatting::fixed_significant_string(
            f64::from(self),
            usize::from(precision),
            SignificantFormat::Precision,
        )
    }
}

impl NativeNumberPredicate for f64 {
    fn native_is_integer(self) -> bool {
        self.is_finite() && self.fract() == 0.0
    }
    fn native_is_safe_integer(self) -> bool {
        self.native_is_integer() && self.abs() <= ((1_u64 << Self::MANTISSA_DIGITS) - 1) as f64
    }
    fn native_is_finite(self) -> bool {
        self.is_finite()
    }
    fn native_is_nan(self) -> bool {
        self.is_nan()
    }
}

impl JsNumberValue for f64 {
    fn is_negative_zero(self) -> bool {
        self == 0.0 && self.is_sign_negative()
    }
    fn to_js_decimal_string(self) -> String {
        ryu_js::Buffer::new().format(self).to_owned()
    }

    fn fixed_string(self, digits: u8) -> String {
        formatting::fixed_string(self, digits)
    }

    fn exponential_string(self, digits: Option<u8>) -> String {
        match digits {
            None => formatting::shortest_exponential(self),
            Some(digits) => formatting::fixed_significant_string(
                self,
                usize::from(digits) + 1,
                SignificantFormat::Exponential,
            ),
        }
    }

    fn precision_string(self, precision: u8) -> String {
        formatting::fixed_significant_string(
            self,
            usize::from(precision),
            SignificantFormat::Precision,
        )
    }
}

pub fn to_string<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn value_of<T: JsNumberValue>(value: T) -> T {
    value
}

pub fn to_string_radix<T: JsIntegerValue>(value: T, radix: f64) -> Result<String, JsError> {
    let radix = u32::from(integer_parameter(radix, 2, 36, "toString radix")?);
    Ok(value.to_js_radix_string(radix))
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

    let mut base = radix.map_or(0, |value| {
        crate::native_integer::Integer32::integer32(value) as i32
    });
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
    let count = source
        .bytes()
        .take_while(|value| ascii_digit(*value).is_some_and(|digit| digit < radix))
        .count();
    if count == 0 {
        return f64::NAN;
    }
    let magnitude = parsing::unsigned_integer(&source[..count], radix);
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

pub fn is_nan<T: NativeNumberPredicate>(value: T) -> bool {
    value.native_is_nan()
}

pub fn is_finite<T: NativeNumberPredicate>(value: T) -> bool {
    value.native_is_finite()
}

pub fn is_integer<T: NativeNumberPredicate>(value: T) -> bool {
    value.native_is_integer()
}

pub fn is_safe_integer<T: NativeNumberPredicate>(value: T) -> bool {
    value.native_is_safe_integer()
}

pub fn to_fixed<T: JsNumberValue>(value: T, digits: Option<f64>) -> Result<String, JsError> {
    let digits = integer_parameter(digits.unwrap_or(0.0), 0, 100, "toFixed digits")?;
    Ok(value.fixed_string(digits))
}

pub fn to_fixed_default<T: JsNumberValue>(value: T) -> String {
    value.fixed_string(0)
}

pub fn to_fixed_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_fixed(value, Some(digits))
}

pub fn to_exponential<T: JsNumberValue>(value: T, digits: Option<f64>) -> Result<String, JsError> {
    let digits = digits
        .map(|value| integer_parameter(value, 0, 100, "toExponential digits"))
        .transpose()?;
    Ok(value.exponential_string(digits))
}

pub fn to_exponential_default<T: JsNumberValue>(value: T) -> String {
    value.exponential_string(None)
}

pub fn to_exponential_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_exponential(value, Some(digits))
}

pub fn to_precision<T: JsNumberValue>(value: T, precision: Option<f64>) -> Result<String, JsError> {
    match precision {
        None => Ok(value.to_js_decimal_string()),
        Some(precision) => Ok(value.precision_string(integer_parameter(
            precision,
            1,
            100,
            "toPrecision precision",
        )?)),
    }
}

pub fn to_precision_default<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn to_precision_digits<T: JsNumberValue>(value: T, precision: f64) -> Result<String, JsError> {
    to_precision(value, Some(precision))
}

fn integer_radix(mut magnitude: u128, negative: bool, radix: u32) -> String {
    let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buffer = [0_u8; 129];
    let mut start = buffer.len();
    loop {
        start -= 1;
        buffer[start] = alphabet[(magnitude % u128::from(radix)) as usize];
        magnitude /= u128::from(radix);
        if magnitude == 0 {
            break;
        }
    }
    if negative {
        start -= 1;
        buffer[start] = b'-';
    }
    std::str::from_utf8(&buffer[start..])
        .expect("native radix digits are ASCII")
        .to_owned()
}

fn fixed_integer(value: impl Display, digits: usize) -> String {
    if digits == 0 {
        return value.to_string();
    }
    let mut result = String::with_capacity(digits + 41);
    write!(result, "{value}.").expect("writing to String is infallible");
    result.extend(std::iter::repeat_n('0', digits));
    result
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

pub(crate) fn is_ecmascript_whitespace(value: char) -> bool {
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
