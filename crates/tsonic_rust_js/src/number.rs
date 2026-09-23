//! Native numeric operations exposed through the source Number API.

mod borrowed;
mod numeric;
mod source_numeric;
pub use borrowed::NumericRef;
pub use numeric::{bigint_to_number, JsNumeric};
pub use source_numeric::SourceNumeric;

use std::fmt::{Display, LowerExp, Write};
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
    fn to_js_decimal_string(self) -> String;

    fn fixed_string(self, digits: usize) -> String {
        format!("{self:.digits$}")
    }
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
        true
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
                fn native_is_safe_integer(self) -> bool { true }
                fn native_is_finite(self) -> bool { true }
                fn native_is_nan(self) -> bool { false }
            }

            impl JsNumberValue for $type {
                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }

                fn fixed_string(self, digits: usize) -> String {
                    fixed_integer(self, digits)
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
                fn native_is_safe_integer(self) -> bool { true }
                fn native_is_finite(self) -> bool { true }
                fn native_is_nan(self) -> bool { false }
            }

            impl JsNumberValue for $type {
                fn to_js_decimal_string(self) -> String {
                    self.to_string()
                }

                fn fixed_string(self, digits: usize) -> String {
                    fixed_integer(self, digits)
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
        self.native_is_integer() && self.abs() <= ((1_u64 << Self::MANTISSA_DIGITS) - 1) as f32
    }
    fn native_is_finite(self) -> bool {
        self.is_finite()
    }
    fn native_is_nan(self) -> bool {
        self.is_nan()
    }
}

impl JsNumberValue for f32 {
    fn to_js_decimal_string(self) -> String {
        self.to_string()
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
    fn to_js_decimal_string(self) -> String {
        self.to_string()
    }
}

pub fn to_string<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn value_of<T: JsNumberValue>(value: T) -> T {
    value
}

pub fn to_string_radix<T: JsIntegerValue>(value: T, radix: f64) -> Result<String, JsError> {
    let radix = native_radix(radix).ok_or_else(|| {
        JsError::new(
            JsErrorKind::RangeError,
            "Number radix must be an integer between 2 and 36",
        )
    })?;
    Ok(value.to_js_radix_string(radix))
}

pub fn parse_int(text: &str, radix: Option<f64>) -> f64 {
    let Some(radix) = native_radix(radix.unwrap_or(10.0)) else {
        return f64::NAN;
    };
    i128::from_str_radix(text, radix).map_or(f64::NAN, |value| value as f64)
}

pub fn parse_int_default(text: &str) -> f64 {
    parse_int(text, None)
}

pub fn parse_int_radix(text: &str, radix: f64) -> f64 {
    parse_int(text, Some(radix))
}

pub fn parse_float(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
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
    let digits = formatting_count(digits.unwrap_or(0.0), 0)?;
    Ok(value.fixed_string(digits))
}

pub fn to_fixed_default<T: JsNumberValue>(value: T) -> String {
    value.fixed_string(0)
}

pub fn to_fixed_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_fixed(value, Some(digits))
}

pub fn to_exponential<T: JsNumberValue>(value: T, digits: Option<f64>) -> Result<String, JsError> {
    match digits {
        None => Ok(format!("{value:e}")),
        Some(digits) => {
            let digits = formatting_count(digits, 0)?;
            Ok(format!("{value:.digits$e}"))
        }
    }
}

pub fn to_exponential_default<T: JsNumberValue>(value: T) -> String {
    format!("{value:e}")
}

pub fn to_exponential_digits<T: JsNumberValue>(value: T, digits: f64) -> Result<String, JsError> {
    to_exponential(value, Some(digits))
}

pub fn to_precision<T: JsNumberValue>(value: T, precision: Option<f64>) -> Result<String, JsError> {
    match precision {
        None => Ok(value.to_js_decimal_string()),
        Some(precision) => {
            let digits = formatting_count(precision, 1)? - 1;
            Ok(format!("{value:.digits$e}"))
        }
    }
}

pub fn to_precision_default<T: JsNumberValue>(value: T) -> String {
    value.to_js_decimal_string()
}

pub fn to_precision_digits<T: JsNumberValue>(value: T, precision: f64) -> Result<String, JsError> {
    to_precision(value, Some(precision))
}

fn formatting_count(value: f64, minimum: usize) -> Result<usize, JsError> {
    let count = crate::native_integer::native_length(value)?;
    if count < minimum || count > (isize::MAX as usize).saturating_sub(41) {
        return Err(JsError::new(
            JsErrorKind::RangeError,
            "Number formatting count exceeds native string capacity",
        ));
    }
    Ok(count)
}

fn native_radix(value: f64) -> Option<u32> {
    (value.is_finite() && value.fract() == 0.0 && (2.0..=36.0).contains(&value))
        .then_some(value as u32)
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
