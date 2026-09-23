//! Math helper module.

use std::sync::atomic::{AtomicU64, Ordering};

static RANDOM_STATE: AtomicU64 = AtomicU64::new(0x9E3779B97F4A7C15);

fn next_random_u64() -> u64 {
    let mut state = RANDOM_STATE.load(Ordering::Relaxed);
    loop {
        let mut next = state;
        next ^= next << 7;
        next ^= next >> 9;
        next ^= next << 8;
        match RANDOM_STATE.compare_exchange_weak(state, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return next,
            Err(actual) => state = actual,
        }
    }
}

pub const E: f64 = std::f64::consts::E;
pub const LN10: f64 = std::f64::consts::LN_10;
pub const LN2: f64 = std::f64::consts::LN_2;
pub const LOG10E: f64 = std::f64::consts::LOG10_E;
pub const LOG2E: f64 = std::f64::consts::LOG2_E;
pub const PI: f64 = std::f64::consts::PI;
pub const SQRT1_2: f64 = std::f64::consts::FRAC_1_SQRT_2;
pub const SQRT2: f64 = std::f64::consts::SQRT_2;

pub fn abs(value: f64) -> f64 {
    value.abs()
}
pub fn acos(value: f64) -> f64 {
    value.acos()
}
pub fn acosh(value: f64) -> f64 {
    value.acosh()
}
pub fn asin(value: f64) -> f64 {
    value.asin()
}
pub fn asinh(value: f64) -> f64 {
    value.asinh()
}
pub fn atan(value: f64) -> f64 {
    value.atan()
}
pub fn atanh(value: f64) -> f64 {
    value.atanh()
}
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}
pub fn cbrt(value: f64) -> f64 {
    value.cbrt()
}
pub fn ceil(value: f64) -> f64 {
    value.ceil()
}
pub fn floor(value: f64) -> f64 {
    value.floor()
}
pub fn clz32(value: i32) -> i32 {
    value.leading_zeros() as i32
}
pub fn cos(value: f64) -> f64 {
    value.cos()
}
pub fn cosh(value: f64) -> f64 {
    value.cosh()
}
pub fn exp(value: f64) -> f64 {
    value.exp()
}
pub fn expm1(value: f64) -> f64 {
    value.exp_m1()
}
pub fn fround(value: f64) -> f64 {
    (value as f32) as f64
}
pub fn hypot(values: &[f64]) -> f64 {
    let mut maximum = 0.0_f64;
    let mut has_nan = false;
    for value in values {
        let absolute = value.abs();
        if absolute.is_infinite() {
            return f64::INFINITY;
        }
        if absolute.is_nan() {
            has_nan = true;
        } else {
            maximum = maximum.max(absolute);
        }
    }
    if has_nan {
        return f64::NAN;
    }
    if maximum == 0.0 {
        return 0.0;
    }

    let mut sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for value in values {
        let scaled = value / maximum;
        let square = scaled * scaled;
        let corrected = square - compensation;
        let next = sum + corrected;
        compensation = (next - sum) - corrected;
        sum = next;
    }
    maximum * sum.sqrt()
}
pub fn imul(left: i32, right: i32) -> i32 {
    left.wrapping_mul(right)
}
pub fn log(value: f64) -> f64 {
    value.ln()
}
pub fn log1p(value: f64) -> f64 {
    value.ln_1p()
}
pub fn log10(value: f64) -> f64 {
    value.log10()
}
pub fn log2(value: f64) -> f64 {
    value.log2()
}
pub fn max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
pub fn min(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}
pub fn pow(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}
pub fn random() -> f64 {
    let bits = next_random_u64();
    (bits as f64) / ((u64::MAX as f64) + 1.0)
}
pub fn round(value: f64) -> f64 {
    value.round()
}
pub fn sign(value: f64) -> f64 {
    value.signum()
}
pub fn sin(value: f64) -> f64 {
    value.sin()
}
pub fn sinh(value: f64) -> f64 {
    value.sinh()
}
pub fn sqrt(value: f64) -> f64 {
    value.sqrt()
}
pub fn tan(value: f64) -> f64 {
    value.tan()
}
pub fn tanh(value: f64) -> f64 {
    value.tanh()
}
pub fn trunc(value: f64) -> f64 {
    value.trunc()
}
