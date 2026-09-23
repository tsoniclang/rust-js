use std::fmt::{self, Write};

#[derive(Clone, Copy)]
pub(super) enum SignificantFormat {
    Exponential,
    Precision,
}

struct TextBuffer {
    bytes: [u8; 128],
    length: usize,
}

impl TextBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 128],
            length: 0,
        }
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.length]).expect("numeric formatting is UTF-8")
    }
}

impl Write for TextBuffer {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .length
            .checked_add(value.len())
            .filter(|end| *end <= self.bytes.len())
            .ok_or(fmt::Error)?;
        self.bytes[self.length..end].copy_from_slice(value.as_bytes());
        self.length = end;
        Ok(())
    }
}

struct DecimalDigits {
    bytes: [u8; 101],
    length: usize,
    exponent: i32,
    negative: bool,
}

impl DecimalDigits {
    fn read(mut source: &str) -> Self {
        let negative = source.starts_with('-');
        if negative {
            source = &source[1..];
        }
        let (mantissa, suffix) = source.split_once('e').unwrap_or((source, "0"));
        let point = mantissa.find('.').unwrap_or(mantissa.len());
        let mut result = Self {
            bytes: [b'0'; 101],
            length: 0,
            exponent: 0,
            negative,
        };
        let mut leading = 0;
        for digit in mantissa.bytes().filter(|digit| *digit != b'.') {
            if result.length == 0 && digit == b'0' {
                leading += 1;
            } else {
                result.bytes[result.length] = digit;
                result.length += 1;
            }
        }
        result.exponent =
            point as i32 - leading - 1 + suffix.parse::<i32>().expect("numeric exponent");
        if result.length == 0 {
            result.length = 1;
            result.exponent = 0;
            result.negative = false;
        }
        result
    }

    fn resize(&mut self, precision: Option<usize>) {
        if let Some(precision) = precision {
            if self.length > precision && self.bytes[precision] >= b'5' {
                let mut cursor = precision;
                while cursor != 0 && self.bytes[cursor - 1] == b'9' {
                    cursor -= 1;
                    self.bytes[cursor] = b'0';
                }
                if cursor == 0 {
                    self.bytes[0] = b'1';
                    self.exponent += 1;
                } else {
                    self.bytes[cursor - 1] += 1;
                }
            }
            if self.length < precision {
                self.bytes[self.length..precision].fill(b'0');
            }
            self.length = precision;
        } else {
            while self.length > 1 && self.bytes[self.length - 1] == b'0' {
                self.length -= 1;
            }
        }
    }

    fn render(&self, format: SignificantFormat) -> String {
        let digits =
            std::str::from_utf8(&self.bytes[..self.length]).expect("numeric digits are ASCII");
        let exponential = matches!(format, SignificantFormat::Exponential)
            || self.exponent < -6
            || self.exponent >= self.length as i32;
        let capacity = if exponential {
            self.length + 8
        } else {
            self.length.max((self.exponent + 1).max(0) as usize)
                + (-self.exponent).max(0) as usize
                + 3
        };
        let mut result = String::with_capacity(capacity);
        if self.negative {
            result.push('-');
        }
        if exponential {
            result.push(char::from(self.bytes[0]));
            if self.length > 1 {
                result.push('.');
                result.push_str(&digits[1..]);
            }
            result.push('e');
            if self.exponent >= 0 {
                result.push('+');
            }
            write!(result, "{}", self.exponent).expect("writing to String");
        } else {
            let point = self.exponent + 1;
            if point <= 0 {
                result.push_str("0.");
                for _ in 0..-point {
                    result.push('0');
                }
                result.push_str(digits);
            } else if point as usize >= self.length {
                result.push_str(digits);
                for _ in self.length..point as usize {
                    result.push('0');
                }
            } else {
                result.push_str(&digits[..point as usize]);
                result.push('.');
                result.push_str(&digits[point as usize..]);
            }
        }
        result
    }
}

fn decimal_half_rounds_down(value: f64, decimals: i32) -> bool {
    if value == 0.0 {
        return false;
    }
    let bits = value.abs().to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let mut significand = bits & ((1_u64 << 52) - 1);
    let exponent = if raw_exponent == 0 {
        -1074
    } else {
        raw_exponent - 1075
    };
    if raw_exponent != 0 {
        significand |= 1_u64 << 52;
    }
    for _ in decimals..0 {
        if significand % 5 != 0 {
            return false;
        }
        significand /= 5;
    }
    let denominator_bits = -(exponent + decimals);
    if !(1..=64).contains(&denominator_bits)
        || significand.trailing_zeros() != (denominator_bits - 1) as u32
    {
        return false;
    }
    (significand >> (denominator_bits - 1)) & 3 == 1
}

pub(super) fn fixed_string(value: f64, precision: u8) -> String {
    if !value.is_finite() || value.abs() >= 1e21 {
        return ryu_js::Buffer::new().format(value).to_owned();
    }
    let value = if value == 0.0 { 0.0 } else { value };
    let precision = usize::from(precision);
    let mut buffer = TextBuffer::new();
    write!(buffer, "{value:.precision$}").expect("bounded fixed representation");
    if decimal_half_rounds_down(value, precision as i32) {
        buffer.bytes[buffer.length - 1] += 1;
    }
    buffer.text().to_owned()
}

pub(super) fn fixed_significant_string(
    value: f64,
    precision: usize,
    format: SignificantFormat,
) -> String {
    if !value.is_finite() {
        return ryu_js::Buffer::new().format(value).to_owned();
    }
    let mut buffer = TextBuffer::new();
    write!(buffer, "{value:e}").expect("bounded floating representation");
    let exponent = DecimalDigits::read(buffer.text()).exponent;
    let increment = decimal_half_rounds_down(value, precision as i32 - 1 - exponent);
    buffer.length = 0;
    let places = precision - 1;
    write!(buffer, "{value:.places$e}").expect("bounded significant representation");
    let mut digits = DecimalDigits::read(buffer.text());
    if increment {
        digits.bytes[digits.length - 1] += 1;
    }
    digits.resize(Some(precision));
    digits.render(format)
}

pub(super) fn shortest_exponential<T: ryu_js::Float>(value: T) -> String {
    let mut buffer = ryu_js::Buffer::new();
    let source = buffer.format(value);
    if matches!(source, "NaN" | "Infinity" | "-Infinity") {
        return source.to_owned();
    }
    let mut digits = DecimalDigits::read(source);
    digits.resize(None);
    digits.render(SignificantFormat::Exponential)
}

pub(super) fn integer_significant(
    value: impl fmt::Display,
    precision: Option<usize>,
    format: SignificantFormat,
) -> String {
    let mut buffer = TextBuffer::new();
    write!(buffer, "{value}").expect("bounded native integer representation");
    let mut digits = DecimalDigits::read(buffer.text());
    digits.resize(precision);
    digits.render(format)
}
