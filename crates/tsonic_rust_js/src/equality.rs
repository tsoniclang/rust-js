//! Equality helpers for JS-compatible comparison behavior.

/// JS SameValueZero comparison.
pub trait JsSameValueZero<Rhs: ?Sized = Self> {
    fn same_value_zero(&self, other: &Rhs) -> bool;
}

pub trait JsSameValue<Rhs: ?Sized = Self> {
    fn same_value(&self, other: &Rhs) -> bool;
}

/// JS strict equality comparison.
pub trait JsStrictEqual<Rhs: ?Sized = Self> {
    fn strict_equal(&self, other: &Rhs) -> bool;
}

/// Stable hash corresponding to JS SameValueZero comparison.
///
/// Equal values must return the same hash. Hash collisions are resolved with
/// [`JsSameValueZero`], so this is an index contract rather than an identity
/// substitute.
pub trait JsHash {
    fn js_hash(&self) -> u64;
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

pub(crate) fn hash_identity(identity: usize) -> u64 {
    hash_bytes(&identity.to_ne_bytes())
}

pub fn same_value_zero_f64(left: f64, right: f64) -> bool {
    if left.is_nan() && right.is_nan() {
        return true;
    }
    left == right
}

pub fn same_value_f64(left: f64, right: f64) -> bool {
    if left.is_nan() && right.is_nan() {
        return true;
    }
    if left == 0.0 && right == 0.0 {
        return left.is_sign_negative() == right.is_sign_negative();
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

impl JsSameValue for f64 {
    fn same_value(&self, other: &Self) -> bool {
        same_value_f64(*self, *other)
    }
}

impl JsStrictEqual for f64 {
    fn strict_equal(&self, other: &Self) -> bool {
        strict_equal_f64(*self, *other)
    }
}

impl JsHash for f64 {
    fn js_hash(&self) -> u64 {
        let bits = if self.is_nan() {
            f64::NAN.to_bits()
        } else if *self == 0.0 {
            0
        } else {
            self.to_bits()
        };
        hash_bytes(&bits.to_ne_bytes())
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

impl JsSameValue for f32 {
    fn same_value(&self, other: &Self) -> bool {
        if self.is_nan() && other.is_nan() {
            return true;
        }
        if *self == 0.0 && *other == 0.0 {
            return self.is_sign_negative() == other.is_sign_negative();
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

impl JsHash for f32 {
    fn js_hash(&self) -> u64 {
        let bits = if self.is_nan() {
            f32::NAN.to_bits()
        } else if *self == 0.0 {
            0
        } else {
            self.to_bits()
        };
        hash_bytes(&bits.to_ne_bytes())
    }
}

impl JsSameValueZero for BigInt {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsSameValue for BigInt {
    fn same_value(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsStrictEqual for BigInt {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for BigInt {
    fn js_hash(&self) -> u64 {
        hash_bytes(&self.to_signed_bytes_le())
    }
}

impl JsSameValueZero for Undefined {
    fn same_value_zero(&self, _other: &Self) -> bool {
        true
    }
}

impl JsSameValue for Undefined {
    fn same_value(&self, _other: &Self) -> bool {
        true
    }
}

impl JsStrictEqual for Undefined {
    fn strict_equal(&self, _other: &Self) -> bool {
        true
    }
}

impl JsHash for Undefined {
    fn js_hash(&self) -> u64 {
        FNV_OFFSET_BASIS
    }
}

impl JsSameValueZero<str> for String {
    fn same_value_zero(&self, other: &str) -> bool {
        self == other
    }
}

impl JsSameValue<str> for String {
    fn same_value(&self, other: &str) -> bool {
        self == other
    }
}

impl JsStrictEqual<str> for String {
    fn strict_equal(&self, other: &str) -> bool {
        self == other
    }
}

impl JsHash for str {
    fn js_hash(&self) -> u64 {
        hash_bytes(self.as_bytes())
    }
}

impl JsHash for String {
    fn js_hash(&self) -> u64 {
        self.as_str().js_hash()
    }
}

impl JsHash for &str {
    fn js_hash(&self) -> u64 {
        (*self).js_hash()
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

            impl JsSameValue for $t {
                fn same_value(&self, other: &Self) -> bool {
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

macro_rules! impl_js_integer_hash {
    ($($t:ty),* $(,)?) => {
        $(
            impl JsHash for $t {
                fn js_hash(&self) -> u64 {
                    hash_bytes(&self.to_ne_bytes())
                }
            }
        )*
    };
}

impl JsHash for bool {
    fn js_hash(&self) -> u64 {
        hash_bytes(&[u8::from(*self)])
    }
}

impl JsHash for char {
    fn js_hash(&self) -> u64 {
        hash_bytes(&u32::from(*self).to_ne_bytes())
    }
}

impl_js_integer_hash!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl_js_primitive_equality!(
    bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, char, String, &str
);
use tsonic_rust_runtime::{BigInt, Undefined};
