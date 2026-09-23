use std::hint::black_box;

use tsonic_rust_js::math;

#[test]
fn constants_are_exposed() {
    let e = black_box(math::E);
    let ln10 = black_box(math::LN10);
    let ln2 = black_box(math::LN2);
    let log10e = black_box(math::LOG10E);
    let log2e = black_box(math::LOG2E);
    let pi = black_box(math::PI);
    let sqrt1_2 = black_box(math::SQRT1_2);
    let sqrt2 = black_box(math::SQRT2);
    assert!(e > 2.0);
    assert!(ln10 > 2.0);
    assert!(ln2 > 0.0);
    assert!(log10e > 0.0);
    assert!(log2e > 1.0);
    assert!(pi > 3.0);
    assert!(sqrt1_2 > 0.0);
    assert!(sqrt2 > 1.0);
}

#[test]
fn math_max_min_with_empty_and_nan() {
    assert_eq!(math::max(&[]), f64::NEG_INFINITY);
    assert_eq!(math::min(&[]), f64::INFINITY);
    assert!(math::max(&[1.0, f64::NAN]).is_nan());
    assert!(math::min(&[1.0, f64::NAN]).is_nan());
    assert!(math::max(&[f64::NAN, 1.0]).is_nan());
    assert!(math::min(&[f64::NAN, 1.0]).is_nan());
    assert_eq!(math::max(&[-0.0, 0.0]), 0.0);
    assert_eq!(math::max(&[0.0, -0.0]), 0.0);
    assert_eq!(math::min(&[-0.0, 0.0]), -0.0);
    assert_eq!(math::min(&[0.0, -0.0]), -0.0);
}

#[test]
fn imul_and_clz32_and_shift_sign() {
    assert_eq!(math::imul(2_147_483_647.0, 2.0), -2);
    assert_eq!(math::imul(f64::NAN, f64::INFINITY), 0);
    assert_eq!(math::imul(4_294_967_297.0, 2.0), 2);
    assert_eq!(math::clz32(1.0), 31);
    assert_eq!(math::clz32(f64::NAN), 32);
    assert_eq!(math::clz32(0.0), 32);
    assert!(math::sign(-0.0).is_sign_negative());
    assert_eq!(math::sign(-12.0), -1.0);
    let random = math::random();
    assert!((0.0..1.0).contains(&random));
}

#[test]
fn basic_math_helpers() {
    assert_eq!(math::abs(-3.0), 3.0);
    assert_eq!(math::acos(1.0), 0.0);
    assert_eq!(math::asin(0.0), 0.0);
    assert_eq!(math::atan(0.0), 0.0);
    assert_eq!(math::atan2(0.0, -1.0), std::f64::consts::PI);
    assert_eq!(math::round(1.5), 2.0);
    assert_eq!(math::round(-0.5), -0.0);
    assert_eq!(math::round(-1.5), -1.0);
    assert_eq!(math::round(-2.5), -2.0);
    assert_eq!(math::trunc(1.9), 1.0);
    assert_eq!(math::trunc(-1.9), -1.0);
    assert_eq!(math::floor(1.9), 1.0);
    assert_eq!(math::ceil(1.1), 2.0);
    assert_eq!(math::cos(0.0), 1.0);
    assert_eq!(math::exp(0.0), 1.0);
    assert_eq!(math::fround(1.5), 1.5);
    assert_eq!(math::fround(1.337), (1.337_f64 as f32) as f64);
    assert_eq!(math::hypot(&[3.0, 4.0]), 5.0);
    assert!(math::hypot(&[f64::NAN, f64::INFINITY]).is_infinite());
    assert!(math::hypot(&[f64::NAN, 1.0]).is_nan());
    assert!(math::hypot(&[f64::MAX / 2.0, f64::MAX / 2.0]).is_finite());
    assert_eq!(math::log(1.0), 0.0);
    assert_eq!(math::pow(2.0, 3.0), 8.0);
    assert!(math::pow(1.0, f64::INFINITY).is_nan());
    assert!(math::pow(-1.0, f64::NEG_INFINITY).is_nan());
    assert_eq!(math::sin(0.0), 0.0);
    assert_eq!(math::sqrt(9.0), 3.0);
    assert_eq!(math::tan(0.0), 0.0);
}

#[test]
fn complete_math_function_family() {
    assert_eq!(math::acosh(1.0), 0.0);
    assert_eq!(math::asinh(0.0), 0.0);
    assert_eq!(math::atanh(0.0), 0.0);
    assert_eq!(math::cbrt(27.0), 3.0);
    assert_eq!(math::cosh(0.0), 1.0);
    assert_eq!(math::expm1(0.0), 0.0);
    assert_eq!(math::log1p(0.0), 0.0);
    assert_eq!(math::log10(100.0), 2.0);
    assert_eq!(math::log2(8.0), 3.0);
    assert_eq!(math::sinh(0.0), 0.0);
    assert_eq!(math::tanh(0.0), 0.0);
}
