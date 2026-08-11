//! UTF-16-aware JS string helpers over valid Rust `str`.

use tsonic_rust_runtime::{JsError, JsErrorKind};

use crate::array::JsArray;
use crate::coercion::{absolute_index, relative_index, to_integer_or_infinity};

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

fn from_units(units: &[u16]) -> Result<String, JsError> {
    String::from_utf16(units).map_err(|_| {
        JsError::new(
            JsErrorKind::Unsupported,
            "a JavaScript string containing an unpaired UTF-16 surrogate requires a UTF-16 string carrier",
        )
    })
}

pub fn js_len(value: &str) -> usize {
    utf16_units(value).len()
}

pub fn char_at(value: &str, index: f64) -> Result<String, JsError> {
    let units = utf16_units(value);
    match absolute_index(index, units.len()) {
        Some(pos) => from_units(&[units[pos]]),
        None => Ok(String::new()),
    }
}

pub fn at(value: &str, index: f64) -> Result<Option<String>, JsError> {
    let units = utf16_units(value);
    relative_index(index, units.len())
        .map(|pos| from_units(&[units[pos]]))
        .transpose()
}

pub fn char_code_at(value: &str, index: f64) -> f64 {
    let units = utf16_units(value);
    absolute_index(index, units.len())
        .map(|pos| units[pos] as f64)
        .unwrap_or(f64::NAN)
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

pub fn slice(value: &str, start: f64, end: Option<f64>) -> Result<String, JsError> {
    let units = utf16_units(value);
    let from = crate::coercion::normalize_slice_index(start, units.len());
    let to = end
        .map(|value| crate::coercion::normalize_slice_index(value, units.len()))
        .unwrap_or(units.len());
    // JS slice does not swap start and end. If normalized start > end, result is empty.
    if from > to {
        return Ok(String::new());
    }
    from_units(&units[from..to])
}

pub fn slice_to(value: &str, start: f64, end: f64) -> Result<String, JsError> {
    slice(value, start, Some(end))
}

fn substring_with_end(value: &str, start: f64, end: Option<f64>) -> Result<String, JsError> {
    let units = utf16_units(value);
    let mut start = clamped_position(start, units.len());
    let mut end = end
        .map(|value| clamped_position(value, units.len()))
        .unwrap_or(units.len());
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    from_units(&units[start..end])
}

pub fn substring(value: &str, start: f64, end: f64) -> Result<String, JsError> {
    substring_with_end(value, start, Some(end))
}

fn substr_with_length(value: &str, start: f64, length: Option<f64>) -> Result<String, JsError> {
    let units = utf16_units(value);
    let start = to_integer_or_infinity(start);
    let start = if start == f64::NEG_INFINITY {
        0
    } else if start < 0.0 {
        (units.len() as f64 + start).max(0.0) as usize
    } else {
        start.min(units.len() as f64) as usize
    };
    let length = length.map(to_integer_or_infinity);
    if length.is_some_and(|value| value <= 0.0) {
        return Ok(String::new());
    }
    let end = length
        .filter(|value| value.is_finite())
        .map(|length| start.saturating_add(length as usize).min(units.len()))
        .unwrap_or(units.len());
    from_units(&units[start..end])
}

pub fn index_of(value: &str, search: &str, position: f64) -> isize {
    let position = clamped_position(position, js_len(value));
    if search.is_empty() {
        return position as isize;
    }
    let haystack = utf16_units(value);
    let needle = utf16_units(search);
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return -1;
    }

    let start = position.min(haystack.len());

    (start..=haystack.len().saturating_sub(needle.len()))
        .find(|&i| haystack[i..i + needle.len()] == needle[..])
        .map(|i| i as isize)
        .unwrap_or(-1)
}

fn last_index_of_with_position(value: &str, search: &str, position: Option<f64>) -> isize {
    let haystack = utf16_units(value);
    let needle = utf16_units(search);
    let position = position
        .map(|value| clamped_position(value, haystack.len()))
        .unwrap_or(haystack.len());
    if needle.is_empty() {
        return position as isize;
    }
    if needle.len() > haystack.len() {
        return -1;
    }

    let max_index = haystack.len() as isize - needle.len() as isize;
    if max_index < 0 {
        return -1;
    }

    let end = position.min(max_index as usize);
    for i in (0..=end).rev() {
        if haystack[i..i + needle.len()] == needle[..] {
            return i as isize;
        }
    }
    -1
}

pub fn starts_with(value: &str, search: &str, position: f64) -> bool {
    let units = utf16_units(value);
    let needle = utf16_units(search);
    let start = clamped_position(position, units.len());
    start + needle.len() <= units.len() && needle == units[start..start + needle.len()]
}

pub fn last_index_of(value: &str, search: &str, position: f64) -> isize {
    last_index_of_with_position(value, search, Some(position))
}

fn ends_with_position(value: &str, search: &str, end_position: Option<f64>) -> bool {
    let units = utf16_units(value);
    let needle = utf16_units(search);
    let end = end_position
        .map(|end| clamped_position(end, units.len()))
        .unwrap_or(units.len());
    if needle.len() > end {
        return false;
    }
    needle == units[end - needle.len()..end]
}

pub fn includes(value: &str, search: &str, position: f64) -> bool {
    index_of(value, search, position) >= 0
}

pub fn includes_from_start(value: &str, search: &str) -> bool {
    includes(value, search, 0.0)
}

pub fn starts_with_from_start(value: &str, search: &str) -> bool {
    starts_with(value, search, 0.0)
}

pub fn ends_with_at_end(value: &str, search: &str) -> bool {
    ends_with_position(value, search, None)
}

pub fn ends_with(value: &str, search: &str, end_position: f64) -> bool {
    ends_with_position(value, search, Some(end_position))
}

pub fn index_of_from_start(value: &str, search: &str) -> isize {
    index_of(value, search, 0.0)
}

pub fn last_index_of_from_end(value: &str, search: &str) -> isize {
    last_index_of_with_position(value, search, None)
}

pub fn substring_from(value: &str, start: f64) -> Result<String, JsError> {
    substring_with_end(value, start, None)
}

pub fn substr_from(value: &str, start: f64) -> Result<String, JsError> {
    substr_with_length(value, start, None)
}

pub fn substr(value: &str, start: f64, length: f64) -> Result<String, JsError> {
    substr_with_length(value, start, Some(length))
}

fn clamped_position(value: f64, length: usize) -> usize {
    let integer = to_integer_or_infinity(value);
    if integer == f64::NEG_INFINITY || integer <= 0.0 {
        0
    } else if integer == f64::INFINITY || integer >= length as f64 {
        length
    } else {
        integer as usize
    }
}

pub fn replace(value: &str, search: &str, replacement: &str) -> String {
    let Some(start) = value.find(search) else {
        return value.to_string();
    };
    let end = start + search.len();
    let mut output = String::new();
    output.push_str(&value[..start]);
    append_replacement(
        &mut output,
        replacement,
        &value[..start],
        &value[start..end],
        &value[end..],
    );
    output.push_str(&value[end..]);
    output
}

pub fn replace_all(value: &str, search: &str, replacement: &str) -> Result<String, JsError> {
    if search.is_empty() {
        if value.chars().any(|character| character.len_utf16() > 1) {
            return Err(JsError::new(
                JsErrorKind::Unsupported,
                "replaceAll with an empty search over astral text requires a UTF-16 string carrier",
            ));
        }
        let mut output = String::new();
        for (start, character) in value.char_indices() {
            append_replacement(
                &mut output,
                replacement,
                &value[..start],
                "",
                &value[start..],
            );
            output.push(character);
        }
        append_replacement(&mut output, replacement, value, "", "");
        return Ok(output);
    }

    let mut output = String::new();
    let mut consumed = 0;
    for (relative_start, _) in value.match_indices(search) {
        if relative_start < consumed {
            continue;
        }
        output.push_str(&value[consumed..relative_start]);
        let end = relative_start + search.len();
        append_replacement(
            &mut output,
            replacement,
            &value[..relative_start],
            &value[relative_start..end],
            &value[end..],
        );
        consumed = end;
    }
    output.push_str(&value[consumed..]);
    Ok(output)
}

fn append_replacement(
    output: &mut String,
    replacement: &str,
    prefix: &str,
    matched: &str,
    suffix: &str,
) {
    let mut chars = replacement.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '$' {
            output.push(character);
            continue;
        }
        match chars.peek().copied() {
            Some('$') => {
                chars.next();
                output.push('$');
            }
            Some('&') => {
                chars.next();
                output.push_str(matched);
            }
            Some('`') => {
                chars.next();
                output.push_str(prefix);
            }
            Some('\'') => {
                chars.next();
                output.push_str(suffix);
            }
            _ => output.push('$'),
        }
    }
}

fn split_with_limit(
    value: &str,
    separator: &str,
    limit: Option<f64>,
) -> Result<JsArray<String>, JsError> {
    let limit = limit.map(to_uint32).unwrap_or(u32::MAX) as usize;
    if limit == 0 {
        return Ok(JsArray::new());
    }
    if separator.is_empty() {
        let parts = utf16_units(value)
            .into_iter()
            .map(|unit| from_units(&[unit]))
            .take(limit)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(JsArray::from_dense(parts));
    }
    let parts = value
        .split(separator)
        .map(ToString::to_string)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(JsArray::from_dense(parts))
}

pub fn split_all(value: &str, separator: &str) -> Result<JsArray<String>, JsError> {
    split_with_limit(value, separator, None)
}

pub fn split(value: &str, separator: &str, limit: f64) -> Result<JsArray<String>, JsError> {
    split_with_limit(value, separator, Some(limit))
}

fn to_uint32(value: f64) -> u32 {
    let integer = to_integer_or_infinity(value);
    if !integer.is_finite() || integer == 0.0 {
        return 0;
    }
    integer.rem_euclid(4_294_967_296.0) as u32
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

pub fn identity(value: &str) -> String {
    value.to_string()
}

pub fn concat(value: &str, strings: &[&str]) -> String {
    let additional = strings.iter().map(|string| string.len()).sum::<usize>();
    let mut output = String::with_capacity(value.len().saturating_add(additional));
    output.push_str(value);
    for string in strings {
        output.push_str(string);
    }
    output
}

pub fn from_char_code(code_units: &[f64]) -> Result<String, JsError> {
    let code_units = code_units
        .iter()
        .map(|value| to_uint32(*value) as u16)
        .collect::<Vec<_>>();
    from_units(&code_units)
}

pub fn from_code_point(code_points: &[f64]) -> Result<String, JsError> {
    let mut out = String::new();
    for value in code_points {
        if !value.is_finite()
            || value.fract() != 0.0
            || *value < 0.0
            || *value > 0x10FFFF as f64
            || (0xD800 as f64..=0xDFFF as f64).contains(value)
        {
            return Err(JsError::new(
                JsErrorKind::RangeError,
                "fromCodePoint expects a value between 0 and 0x10FFFF excluding surrogate code points",
            ));
        }
        if let Some(ch) = std::char::from_u32(*value as u32) {
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
