use num_bigint::BigUint;
use num_traits::Zero;

use super::{integer_option, range_error, JsResult, JsValue};

#[derive(Clone, Debug)]
pub(super) struct NumberPrecision {
    pub minimum_integer: u16,
    pub minimum_fraction: Option<u16>,
    pub maximum_fraction: Option<u16>,
    pub minimum_significant: Option<u16>,
    pub maximum_significant: Option<u16>,
}

impl NumberPrecision {
    pub fn new(options: &JsValue, default_minimum: u16, default_maximum: u16) -> JsResult<Self> {
        let minimum_integer = integer_option(options, "minimumIntegerDigits", 1, 21)?.unwrap_or(1);
        let minimum_significant = integer_option(options, "minimumSignificantDigits", 1, 21)?;
        let maximum_significant = integer_option(options, "maximumSignificantDigits", 1, 21)?;
        let (minimum_fraction, maximum_fraction, minimum_significant, maximum_significant) =
            if minimum_significant.is_some() || maximum_significant.is_some() {
                let minimum = minimum_significant.unwrap_or(1);
                let maximum = maximum_significant.unwrap_or(21);
                if minimum > maximum {
                    return Err(range_error(
                        "Minimum significant digits exceed maximum significant digits",
                    ));
                }
                (None, None, Some(minimum), Some(maximum))
            } else {
                let minimum_fraction = integer_option(options, "minimumFractionDigits", 0, 100)?;
                let maximum_fraction = integer_option(options, "maximumFractionDigits", 0, 100)?;
                let minimum = minimum_fraction
                    .unwrap_or(default_minimum.min(maximum_fraction.unwrap_or(default_maximum)));
                let maximum = maximum_fraction.unwrap_or(default_maximum.max(minimum));
                if minimum > maximum {
                    return Err(range_error(
                        "Minimum fraction digits exceed maximum fraction digits",
                    ));
                }
                (Some(minimum), Some(maximum), None, None)
            };
        Ok(Self {
            minimum_integer,
            minimum_fraction,
            maximum_fraction,
            minimum_significant,
            maximum_significant,
        })
    }

    pub fn format(&self, magnitude: &str, percent: bool) -> (String, String) {
        let (mantissa, mut exponent) =
            magnitude
                .split_once(['e', 'E'])
                .map_or((magnitude, 0_i32), |(value, power)| {
                    (
                        value,
                        power.parse::<i32>().expect("native numeric exponent"),
                    )
                });
        if let Some(point) = mantissa.find('.') {
            exponent -= (mantissa.len() - point - 1) as i32;
        }
        let mut coefficient = BigUint::parse_bytes(mantissa.replace('.', "").as_bytes(), 10)
            .expect("native numeric digits");
        if percent {
            exponent += 2;
        }
        if coefficient.is_zero() {
            exponent = 0;
        }
        let mut digits = coefficient.to_str_radix(10);
        let quantum = self.maximum_significant.map_or_else(
            || -i32::from(self.maximum_fraction.expect("fraction precision")),
            |maximum| digits.len() as i32 + exponent - i32::from(maximum),
        );
        if quantum > exponent {
            let divisor = BigUint::from(10_u8).pow((quantum - exponent) as u32);
            let remainder = &coefficient % &divisor;
            coefficient /= &divisor;
            if remainder * 2_u8 >= divisor {
                coefficient += 1_u8;
            }
            exponent = quantum;
            digits = coefficient.to_str_radix(10);
        }
        let point = digits.len() as i32 + exponent;
        let (integer, mut fraction) = if point <= 0 {
            ("0".to_owned(), "0".repeat((-point) as usize) + &digits)
        } else if point as usize >= digits.len() {
            let padding = "0".repeat(point as usize - digits.len());
            (digits + &padding, String::new())
        } else {
            (
                digits[..point as usize].to_owned(),
                digits[point as usize..].to_owned(),
            )
        };
        fraction.truncate(fraction.trim_end_matches('0').len());
        if let Some(minimum) = self.minimum_significant {
            let significant = format!("{integer}{fraction}")
                .trim_start_matches('0')
                .len()
                .max(1);
            fraction.push_str(&"0".repeat(usize::from(minimum).saturating_sub(significant)));
        } else {
            fraction.push_str(
                &"0".repeat(
                    usize::from(self.minimum_fraction.expect("fraction precision"))
                        .saturating_sub(fraction.len()),
                ),
            );
        }
        let padding = "0".repeat(usize::from(self.minimum_integer).saturating_sub(integer.len()));
        (padding + &integer, fraction)
    }
}
