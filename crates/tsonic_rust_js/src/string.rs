//! UTF-16-aware JS string helpers over valid Rust `str`.

use tsonic_rust_runtime::{JsError, JsErrorKind};

use crate::coercion::{absolute_index, relative_index};

/// JS-facing string value conversion contract used by dense array join and future array helpers.
pub trait JsToString {
    fn to_js_string(&self) -> String;
}

macro_rules! impl_js_to_string {
    ($($type:ty),+ $(,)?) => {
        $(impl JsToString for $type {
            fn to_js_string(&self) -> String {
                self.to_string()
            }
        })+
    };
}

impl_js_to_string!(bool, i8, u8, i16, u16, i32, u32, i64, u64, String);

impl JsToString for str {
    fn to_js_string(&self) -> String {
        self.to_string()
    }
}

fn utf16_units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn from_units(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

pub fn js_len(value: &str) -> usize {
    utf16_units(value).len()
}

pub fn char_at(value: &str, index: f64) -> String {
    let units = utf16_units(value);
    match absolute_index(index, units.len()) {
        Some(pos) => from_units(&[units[pos]]),
        None => String::new(),
    }
}

pub fn at(value: &str, index: f64) -> Option<String> {
    let units = utf16_units(value);
    relative_index(index, units.len()).map(|pos| from_units(&[units[pos]]))
}

pub fn char_code_at(value: &str, index: f64) -> Option<f64> {
    let units = utf16_units(value);
    absolute_index(index, units.len()).map(|pos| units[pos] as f64)
}

pub fn code_point_at(value: &str, index: f64) -> Option<f64> {
    let units = utf16_units(value);
    let pos = absolute_index(index, units.len())?;
    let first = units[pos];
    if (0xD800..=0xDBFF).contains(&first) && pos + 1 < units.len() {
        let second = units[pos + 1];
        if (0xDC00..=0xDFFF).contains(&second) {
            let pair = (u32::from(first - 0xD800) << 10) + u32::from(second - 0xDC00) + 0x10000;
            return Some(f64::from(pair));
        }
    }
    Some(f64::from(first))
}

pub fn slice(value: &str, start: f64, end: Option<f64>) -> String {
    let units = utf16_units(value);
    let from = crate::coercion::normalize_slice_index(start, units.len());
    let to = end
        .map(|value| crate::coercion::normalize_slice_index(value, units.len()))
        .unwrap_or(units.len());
    // JS slice does not swap start and end. If normalized start > end, result is empty.
    if from > to {
        return String::new();
    }
    from_units(&units[from..to])
}

pub fn slice_to(value: &str, start: f64, end: f64) -> String {
    slice(value, start, Some(end))
}

pub fn substring(value: &str, start: isize, end: Option<isize>) -> String {
    let units = utf16_units(value);
    let len = units.len() as isize;
    let mut start = start.max(0).min(len);
    let mut end = end.unwrap_or(len).max(0).min(len);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    from_units(&units[start as usize..end as usize])
}

pub fn substr(value: &str, start: isize, length: Option<usize>) -> String {
    let units = utf16_units(value);
    if units.is_empty() {
        return String::new();
    }
    let start = if start < 0 {
        (units.len() as isize + start).max(0) as usize
    } else {
        (start as usize).min(units.len())
    };
    let end = length
        .map(|length| start.saturating_add(length).min(units.len()))
        .unwrap_or(units.len());
    from_units(&units[start..end])
}

pub fn index_of(value: &str, search: &str, position: isize) -> isize {
    if search.is_empty() {
        let len = js_len(value) as isize;
        return position.max(0).min(len);
    }
    let haystack = utf16_units(value);
    let needle = utf16_units(search);
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return -1;
    }

    let start = position.max(0).min(haystack.len() as isize) as usize;

    (start..=haystack.len().saturating_sub(needle.len()))
        .find(|&i| haystack[i..i + needle.len()] == needle[..])
        .map(|i| i as isize)
        .unwrap_or(-1)
}

pub fn last_index_of(value: &str, search: &str, position: Option<isize>) -> isize {
    let haystack = utf16_units(value);
    let needle = utf16_units(search);
    if needle.is_empty() {
        return if haystack.is_empty() {
            0
        } else {
            haystack.len() as isize
        };
    }
    if needle.len() > haystack.len() {
        return -1;
    }

    let max_index = haystack.len() as isize - needle.len() as isize;
    if max_index < 0 {
        return -1;
    }

    let pos = position.unwrap_or((haystack.len() as isize) - 1);
    let mut start = if pos < 0 { 0 } else { pos };
    if start > max_index {
        start = max_index;
    }

    let end = start as usize;
    for i in (0..=end).rev() {
        if haystack[i..i + needle.len()] == needle[..] {
            return i as isize;
        }
    }
    -1
}

pub fn starts_with(value: &str, search: &str, position: isize) -> bool {
    let units = utf16_units(value);
    let needle = utf16_units(search);
    if search.is_empty() {
        if position <= units.len() as isize {
            return true;
        }
        return false;
    }
    let start = if position < 0 {
        0
    } else if position as usize >= units.len() {
        return false;
    } else {
        position as usize
    };
    start + needle.len() <= units.len() && needle == units[start..start + needle.len()]
}

pub fn ends_with(value: &str, search: &str, end_position: Option<isize>) -> bool {
    let units = utf16_units(value);
    let needle = utf16_units(search);
    let end = end_position
        .map(|end| {
            if end < 0 {
                0
            } else if end as usize > units.len() {
                units.len()
            } else {
                end as usize
            }
        })
        .unwrap_or(units.len());
    if needle.len() > end {
        return false;
    }
    needle == units[end - needle.len()..end]
}

pub fn includes(value: &str, search: &str, position: isize) -> bool {
    index_of(value, search, position) >= 0
}

pub fn replace(value: &str, search: &str, replacement: &str) -> String {
    if search.is_empty() {
        let mut out = replacement.to_string();
        out.push_str(value);
        return out;
    }
    value.replacen(search, replacement, 1)
}

pub fn split(value: &str, separator: &str, limit: Option<usize>) -> Vec<String> {
    if separator.is_empty() {
        let mut parts = utf16_units(value)
            .into_iter()
            .map(|unit| from_units(&[unit]))
            .collect::<Vec<_>>();
        if let Some(limit) = limit {
            parts.truncate(limit);
        }
        return parts;
    }
    let mut parts = value
        .split(separator)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if let Some(limit) = limit {
        parts.truncate(limit);
    }
    parts
}

pub fn repeat(value: &str, count: f64) -> Result<String, JsError> {
    let count = crate::coercion::to_integer_or_infinity(count);
    if count < 0.0 || count == f64::INFINITY {
        return Err(JsError::new(
            JsErrorKind::RangeError,
            "repeat count must be non-negative",
        ));
    }
    if count == 0.0 {
        return Ok(String::new());
    }
    if value.is_empty() {
        return Ok(String::new());
    }
    let value_units = js_len(value) as u64;
    let count = count as u64;
    if value_units
        .checked_mul(count)
        .is_none_or(|length| length > MAX_STRING_UTF16_UNITS)
    {
        return Err(JsError::new(
            JsErrorKind::RangeError,
            "repeat result exceeds the supported string length",
        ));
    }
    Ok(value.repeat(count as usize))
}

pub const MAX_STRING_UTF16_UNITS: u64 = 16_777_216;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub fn pad_start(value: &str, target_length: f64) -> Result<String, JsError> {
    pad(value, target_length, None, true)
}

pub fn pad_start_with(value: &str, target_length: f64, filler: &str) -> Result<String, JsError> {
    pad(value, target_length, Some(filler), true)
}

pub fn pad_end(value: &str, target_length: f64) -> Result<String, JsError> {
    pad(value, target_length, None, false)
}

pub fn pad_end_with(value: &str, target_length: f64, filler: &str) -> Result<String, JsError> {
    pad(value, target_length, Some(filler), false)
}

fn pad(
    value: &str,
    target_length: f64,
    filler: Option<&str>,
    at_start: bool,
) -> Result<String, JsError> {
    let target_length = to_length(target_length);
    let value_units = utf16_units(value);
    if target_length <= value_units.len() as u64 {
        return Ok(value.to_string());
    }
    let filler = filler.unwrap_or(" ");
    if filler.is_empty() {
        return Ok(value.to_string());
    }
    if target_length > MAX_STRING_UTF16_UNITS {
        return Err(JsError::new(
            JsErrorKind::RangeError,
            "invalid string length",
        ));
    }
    let target_length = target_length as usize;
    let needed = target_length - value_units.len();
    let filler_units = utf16_units(filler);
    let mut padding = Vec::new();
    padding
        .try_reserve_exact(needed)
        .map_err(|_| JsError::new(JsErrorKind::RangeError, "invalid string length"))?;
    let repetitions = needed / filler_units.len();
    let remainder = needed % filler_units.len();
    for _ in 0..repetitions {
        padding.extend_from_slice(&filler_units);
    }
    padding.extend_from_slice(&filler_units[..remainder]);

    let mut output = Vec::new();
    output
        .try_reserve_exact(target_length)
        .map_err(|_| JsError::new(JsErrorKind::RangeError, "invalid string length"))?;
    if at_start {
        output.extend_from_slice(&padding);
        output.extend_from_slice(&value_units);
    } else {
        output.extend_from_slice(&value_units);
        output.extend_from_slice(&padding);
    }
    String::from_utf16(&output).map_err(|_| {
        JsError::new(
            JsErrorKind::Unsupported,
            "padding that produces a lone UTF-16 surrogate requires a UTF-16 string carrier",
        )
    })
}

fn to_length(value: f64) -> u64 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    if value.is_infinite() || value >= MAX_SAFE_INTEGER as f64 {
        return MAX_SAFE_INTEGER;
    }
    value.floor() as u64
}

pub fn trim(value: &str) -> String {
    value.trim().to_string()
}
pub fn trim_start(value: &str) -> String {
    value.trim_start().to_string()
}
pub fn trim_end(value: &str) -> String {
    value.trim_end().to_string()
}

pub fn to_lower_case(value: &str) -> String {
    value.to_lowercase()
}
pub fn to_upper_case(value: &str) -> String {
    value.to_uppercase()
}

pub fn from_char_code(code_units: &[u16]) -> String {
    from_units(code_units)
}

pub fn from_code_point(code_points: &[u32]) -> Result<String, JsError> {
    let mut out = String::new();
    for value in code_points {
        if (0xD800..=0xDFFF).contains(value) || *value > 0x10FFFF {
            return Err(JsError::new(
                JsErrorKind::RangeError,
                "fromCodePoint expects a value between 0 and 0x10FFFF excluding surrogate code points",
            ));
        }
        if let Some(ch) = std::char::from_u32(*value) {
            out.push(ch);
        } else {
            return Err(JsError::new(
                JsErrorKind::RangeError,
                "invalid Unicode code point",
            ));
        }
    }
    Ok(out)
}

pub fn raw(raw_parts: &[&str], substitutions: &[&str]) -> String {
    let mut out = String::new();
    for (index, part) in raw_parts.iter().enumerate() {
        out.push_str(part);
        if let Some(value) = substitutions.get(index) {
            out.push_str(value);
        }
    }
    out
}
