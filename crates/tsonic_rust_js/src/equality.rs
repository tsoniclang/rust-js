//! Equality helpers for JS-compatible comparison behavior.

/// JS SameValueZero comparison.
pub trait JsSameValueZero<Rhs: ?Sized = Self> {
    fn same_value_zero(&self, other: &Rhs) -> bool;
}

/// JS strict equality comparison.
pub trait JsStrictEqual<Rhs: ?Sized = Self> {
    fn strict_equal(&self, other: &Rhs) -> bool;
}

pub fn same_value_zero_f64(left: f64, right: f64) -> bool {
    if left.is_nan() && right.is_nan() {
        return true;
    }
    left == right
}

pub fn strict_equal_f64(left: f64, right: f64) -> bool {
    if left.is_nan() || right.is_nan() {
        return false;
    }
    left == right
}

impl JsSameValueZero for f64 {
    fn same_value_zero(&self, other: &Self) -> bool {
        same_value_zero_f64(*self, *other)
    }
}

impl JsStrictEqual for f64 {
    fn strict_equal(&self, other: &Self) -> bool {
        strict_equal_f64(*self, *other)
    }
}

impl JsSameValueZero for f32 {
    fn same_value_zero(&self, other: &Self) -> bool {
        if self.is_nan() && other.is_nan() {
            return true;
        }
        self == other
    }
}

impl JsStrictEqual for f32 {
    fn strict_equal(&self, other: &Self) -> bool {
        if self.is_nan() || other.is_nan() {
            return false;
        }
        self == other
    }
}

impl JsSameValueZero for BigInt {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsStrictEqual for BigInt {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsSameValueZero for Undefined {
    fn same_value_zero(&self, _other: &Self) -> bool {
        true
    }
}

impl JsStrictEqual for Undefined {
    fn strict_equal(&self, _other: &Self) -> bool {
        true
    }
}

impl JsSameValueZero<str> for String {
    fn same_value_zero(&self, other: &str) -> bool {
        self == other
    }
}

impl JsStrictEqual<str> for String {
    fn strict_equal(&self, other: &str) -> bool {
        self == other
    }
}

macro_rules! impl_js_primitive_equality {
    ($($t:ty),* $(,)?) => {
        $(
            impl JsSameValueZero for $t {
                fn same_value_zero(&self, other: &Self) -> bool {
                    self == other
                }
            }

            impl JsStrictEqual for $t {
                fn strict_equal(&self, other: &Self) -> bool {
                    self == other
                }
            }
        )*
    };
}

impl_js_primitive_equality!(
    bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, char, String, &str
);
use tsonic_rust_runtime::{BigInt, Undefined};
