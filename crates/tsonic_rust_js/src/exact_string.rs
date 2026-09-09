use unicode_normalization::UnicodeNormalization;

use crate::array::JsArray;
use crate::coercion::{absolute_index, relative_index, to_integer_or_infinity, to_length};
use crate::errors::{range_error, type_error, JsResult};
use crate::regexp::string_replacement_arguments;
use crate::{JsString, JsValue};

pub fn js_len(value: &JsString) -> usize {
    value.len()
}

pub fn char_at(value: &JsString, index: f64) -> JsString {
    absolute_index(index, value.len())
        .map(|position| value.slice(position..position + 1))
        .unwrap_or_default()
}

pub fn at(value: &JsString, index: f64) -> Option<JsString> {
    relative_index(index, value.len()).map(|position| value.slice(position..position + 1))
}

pub fn char_code_at(value: &JsString, index: f64) -> f64 {
    absolute_index(index, value.len())
        .and_then(|position| value.code_unit_at(position))
        .map(f64::from)
        .unwrap_or(f64::NAN)
}

pub fn code_point_at(value: &JsString, index: f64) -> Option<f64> {
    let position = absolute_index(index, value.len())?;
    value.code_point_at(position).map(f64::from)
}

pub fn slice(value: &JsString, start: f64, end: Option<f64>) -> JsString {
    let from = crate::coercion::normalize_slice_index(start, value.len());
    let to = end
        .map(|end| crate::coercion::normalize_slice_index(end, value.len()))
        .unwrap_or(value.len());
    if from >= to {
        JsString::new()
    } else {
        value.slice(from..to)
    }
}

pub fn slice_to(value: &JsString, start: f64, end: f64) -> JsString {
    slice(value, start, Some(end))
}

fn substring_with_end(value: &JsString, start: f64, end: Option<f64>) -> JsString {
    let mut from = clamped_position(start, value.len());
    let mut to = end
        .map(|end| clamped_position(end, value.len()))
        .unwrap_or(value.len());
    if from > to {
        std::mem::swap(&mut from, &mut to);
    }
    value.slice(from..to)
}

pub fn substring(value: &JsString, start: f64, end: f64) -> JsString {
    substring_with_end(value, start, Some(end))
}

pub fn substring_from(value: &JsString, start: f64) -> JsString {
    substring_with_end(value, start, None)
}

fn substr_with_length(value: &JsString, start: f64, length: Option<f64>) -> JsString {
    let start = to_integer_or_infinity(start);
    let from = if start == f64::NEG_INFINITY {
        0
    } else if start < 0.0 {
        (value.len() as f64 + start).max(0.0) as usize
    } else {
        start.min(value.len() as f64) as usize
    };
    let length = length.map(to_integer_or_infinity);
    if length.is_some_and(|length| length <= 0.0) {
        return JsString::new();
    }
    let to = length
        .filter(|length| length.is_finite())
        .map(|length| from.saturating_add(length as usize).min(value.len()))
        .unwrap_or(value.len());
    value.slice(from..to)
}

pub fn substr_from(value: &JsString, start: f64) -> JsString {
    substr_with_length(value, start, None)
}

pub fn substr(value: &JsString, start: f64, length: f64) -> JsString {
    substr_with_length(value, start, Some(length))
}

pub fn index_of(value: &JsString, search: &JsString, position: f64) -> isize {
    let position = clamped_position(position, value.len());
    find_units(value.units(), search.units(), position)
        .map(|index| index as isize)
        .unwrap_or(-1)
}

fn last_index_of_with_position(
    value: &JsString,
    search: &JsString,
    position: Option<f64>,
) -> isize {
    let position = position
        .map(|position| clamped_position(position, value.len()))
        .unwrap_or(value.len());
    if search.is_empty() {
        return position as isize;
    }
    if search.len() > value.len() {
        return -1;
    }
    let end = position.min(value.len() - search.len());
    (0..=end)
        .rev()
        .find(|index| value.units()[*index..*index + search.len()] == *search.units())
        .map(|index| index as isize)
        .unwrap_or(-1)
}

pub fn starts_with(value: &JsString, search: &JsString, position: f64) -> bool {
    value.starts_with_at(clamped_position(position, value.len()), search)
}

pub fn ends_with(value: &JsString, search: &JsString, end_position: f64) -> bool {
    ends_with_position(value, search, Some(end_position))
}

fn ends_with_position(value: &JsString, search: &JsString, end_position: Option<f64>) -> bool {
    let end = end_position
        .map(|position| clamped_position(position, value.len()))
        .unwrap_or(value.len());
    search.len() <= end && value.units()[end - search.len()..end] == *search.units()
}

pub fn includes(value: &JsString, search: &JsString, position: f64) -> bool {
    index_of(value, search, position) >= 0
}

pub fn includes_from_start(value: &JsString, search: &JsString) -> bool {
    includes(value, search, 0.0)
}

pub fn starts_with_from_start(value: &JsString, search: &JsString) -> bool {
    starts_with(value, search, 0.0)
}

pub fn ends_with_at_end(value: &JsString, search: &JsString) -> bool {
    ends_with_position(value, search, None)
}

pub fn index_of_from_start(value: &JsString, search: &JsString) -> isize {
    index_of(value, search, 0.0)
}

pub fn last_index_of(value: &JsString, search: &JsString, position: f64) -> isize {
    last_index_of_with_position(value, search, Some(position))
}

pub fn last_index_of_from_end(value: &JsString, search: &JsString) -> isize {
    last_index_of_with_position(value, search, None)
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

pub fn replace(value: &JsString, search: &JsString, replacement: &JsString) -> JsString {
    let Some(start) = find_units(value.units(), search.units(), 0) else {
        return value.clone();
    };
    replace_at(value, start, search.len(), replacement)
}

pub fn replace_with<F>(value: &JsString, search: &JsString, replacer: F) -> JsString
where
    F: Fn(JsArray<JsValue>) -> JsString,
{
    match replace_with_core(value, search, false, |arguments| {
        Ok::<JsString, std::convert::Infallible>(replacer(arguments))
    }) {
        Ok(result) => result,
        Err(error) => match error {},
    }
}

pub fn try_replace_with<E, F>(
    value: &JsString,
    search: &JsString,
    replacer: F,
) -> Result<JsString, E>
where
    F: Fn(JsArray<JsValue>) -> Result<JsString, E>,
{
    replace_with_core(value, search, false, replacer)
}

pub fn replace_all(value: &JsString, search: &JsString, replacement: &JsString) -> JsString {
    let mut parts = Vec::new();
    let mut consumed = 0;
    if search.is_empty() {
        for position in 0..=value.len() {
            parts.push(substitution(value, position, 0, replacement));
            if position < value.len() {
                parts.push(value.slice(position..position + 1));
            }
        }
        return JsString::concat(&parts);
    }
    while let Some(start) = find_units(value.units(), search.units(), consumed) {
        parts.push(value.slice(consumed..start));
        parts.push(substitution(value, start, search.len(), replacement));
        consumed = start + search.len();
    }
    parts.push(value.slice(consumed..value.len()));
    JsString::concat(&parts)
}

pub fn replace_all_with<F>(value: &JsString, search: &JsString, replacer: F) -> JsString
where
    F: Fn(JsArray<JsValue>) -> JsString,
{
    match replace_with_core(value, search, true, |arguments| {
        Ok::<JsString, std::convert::Infallible>(replacer(arguments))
    }) {
        Ok(result) => result,
        Err(error) => match error {},
    }
}

pub fn try_replace_all_with<E, F>(
    value: &JsString,
    search: &JsString,
    replacer: F,
) -> Result<JsString, E>
where
    F: Fn(JsArray<JsValue>) -> Result<JsString, E>,
{
    replace_with_core(value, search, true, replacer)
}

fn replace_with_core<E, F>(
    value: &JsString,
    search: &JsString,
    replace_all: bool,
    replacer: F,
) -> Result<JsString, E>
where
    F: Fn(JsArray<JsValue>) -> Result<JsString, E>,
{
    let mut parts = Vec::new();
    if search.is_empty() {
        if !replace_all {
            return Ok(JsString::concat(&[
                replacer(string_replacement_arguments(search, 0, value))?,
                value.clone(),
            ]));
        }
        for position in 0..=value.len() {
            parts.push(replacer(string_replacement_arguments(
                search, position, value,
            ))?);
            if position < value.len() {
                parts.push(value.slice(position..position + 1));
            }
        }
        return Ok(JsString::concat(&parts));
    }
    let mut consumed = 0;
    while let Some(start) = find_units(value.units(), search.units(), consumed) {
        parts.push(value.slice(consumed..start));
        parts.push(replacer(string_replacement_arguments(
            search, start, value,
        ))?);
        consumed = start + search.len();
        if !replace_all {
            break;
        }
    }
    if parts.is_empty() {
        return Ok(value.clone());
    }
    parts.push(value.slice(consumed..value.len()));
    Ok(JsString::concat(&parts))
}

fn replace_at(
    value: &JsString,
    start: usize,
    matched_length: usize,
    replacement: &JsString,
) -> JsString {
    JsString::concat(&[
        value.slice(0..start),
        substitution(value, start, matched_length, replacement),
        value.slice(start + matched_length..value.len()),
    ])
}

fn substitution(
    input: &JsString,
    start: usize,
    matched_length: usize,
    replacement: &JsString,
) -> JsString {
    let mut output = Vec::with_capacity(replacement.len());
    let units = replacement.units();
    let mut index = 0;
    while index < units.len() {
        if units[index] != b'$' as u16 || index + 1 >= units.len() {
            output.push(units[index]);
            index += 1;
            continue;
        }
        match units[index + 1] {
            value if value == b'$' as u16 => {
                output.push(b'$' as u16);
                index += 2;
            }
            value if value == b'&' as u16 => {
                output.extend_from_slice(&input.units()[start..start + matched_length]);
                index += 2;
            }
            value if value == b'`' as u16 => {
                output.extend_from_slice(&input.units()[..start]);
                index += 2;
            }
            value if value == b'\'' as u16 => {
                output.extend_from_slice(&input.units()[start + matched_length..]);
                index += 2;
            }
            _ => {
                output.push(b'$' as u16);
                index += 1;
            }
        }
    }
    JsString::from_units(output)
}

fn split_with_limit(
    value: &JsString,
    separator: &JsString,
    limit: Option<f64>,
) -> JsArray<JsString> {
    let limit = limit.map(to_uint32).unwrap_or(u32::MAX) as usize;
    if limit == 0 {
        return JsArray::new();
    }
    if separator.is_empty() {
        let parts = (0..value.len())
            .map(|index| value.slice(index..index + 1))
            .take(limit)
            .collect();
        return JsArray::from_dense(parts);
    }
    let mut parts = Vec::new();
    let mut consumed = 0;
    while parts.len() < limit {
        let Some(start) = find_units(value.units(), separator.units(), consumed) else {
            break;
        };
        parts.push(value.slice(consumed..start));
        consumed = start + separator.len();
    }
    if parts.len() < limit {
        parts.push(value.slice(consumed..value.len()));
    }
    JsArray::from_dense(parts)
}

pub fn split_all(value: &JsString, separator: &JsString) -> JsArray<JsString> {
    split_with_limit(value, separator, None)
}

pub fn split(value: &JsString, separator: &JsString, limit: f64) -> JsArray<JsString> {
    split_with_limit(value, separator, Some(limit))
}

fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        0
    } else {
        value.trunc().rem_euclid(4_294_967_296.0) as u32
    }
}

pub fn repeat(value: &JsString, count: f64) -> JsResult<JsString> {
    let (_, length) = crate::string_capacity::repeat_shape(value.len(), count)?;
    if length == 0 {
        return Ok(JsString::new());
    }
    let mut units = Vec::new();
    units
        .try_reserve_exact(length)
        .map_err(|_| range_error("invalid string length"))?;
    units.extend_from_slice(value.units());
    while units.len() < length {
        let copied = (length - units.len()).min(units.len());
        units.extend_from_within(..copied);
    }
    Ok(JsString::from_units(units))
}

pub fn pad_start(value: &JsString, target_length: f64) -> JsResult<JsString> {
    pad(value, target_length, None, true)
}

pub fn pad_start_with(
    value: &JsString,
    target_length: f64,
    filler: &JsString,
) -> JsResult<JsString> {
    pad(value, target_length, Some(filler), true)
}

pub fn pad_end(value: &JsString, target_length: f64) -> JsResult<JsString> {
    pad(value, target_length, None, false)
}

pub fn pad_end_with(value: &JsString, target_length: f64, filler: &JsString) -> JsResult<JsString> {
    pad(value, target_length, Some(filler), false)
}

fn pad(
    value: &JsString,
    target_length: f64,
    filler: Option<&JsString>,
    at_start: bool,
) -> JsResult<JsString> {
    let target_length = to_length(target_length);
    if target_length <= value.len() as u64 {
        return Ok(value.clone());
    }
    let default_filler = JsString::from_utf8(" ");
    let filler = filler.unwrap_or(&default_filler);
    if filler.is_empty() {
        return Ok(value.clone());
    }
    let target_length =
        usize::try_from(target_length).map_err(|_| range_error("invalid string length"))?;
    let needed = target_length - value.len();
    let mut output = Vec::new();
    output
        .try_reserve_exact(target_length)
        .map_err(|_| range_error("invalid string length"))?;
    if !at_start {
        output.extend_from_slice(value.units());
    }
    let mut remaining = needed;
    while remaining > 0 {
        let copied = remaining.min(filler.len());
        output.extend_from_slice(&filler.units()[..copied]);
        remaining -= copied;
    }
    if at_start {
        output.extend_from_slice(value.units());
    }
    Ok(JsString::from_units(output))
}

pub fn trim(value: &JsString) -> JsString {
    trim_units(value, true, true)
}

pub fn trim_start(value: &JsString) -> JsString {
    trim_units(value, true, false)
}

pub fn trim_end(value: &JsString) -> JsString {
    trim_units(value, false, true)
}

fn trim_units(value: &JsString, start: bool, end: bool) -> JsString {
    let mut from = 0;
    let mut to = value.len();
    if start {
        while from < to {
            let code_point = value.code_point_at(from).unwrap();
            if !is_ecmascript_whitespace(code_point) {
                break;
            }
            from = value.advance_index(from, true);
        }
    }
    if end {
        while to > from {
            let (code_point, width) = previous_code_point(value.units(), to);
            if !is_ecmascript_whitespace(code_point) {
                break;
            }
            to -= width;
        }
    }
    value.slice(from..to)
}

pub fn to_lower_case(value: &JsString) -> JsString {
    transform_scalar_runs(value, |text| text.to_lowercase())
}

pub fn to_upper_case(value: &JsString) -> JsString {
    transform_scalar_runs(value, |text| text.to_uppercase())
}

pub fn identity(value: &JsString) -> JsString {
    value.clone()
}

pub fn normalize(value: &JsString) -> JsString {
    transform_scalar_runs(value, |text| text.nfc().collect())
}

pub fn normalize_with_form(value: &JsString, form: &str) -> JsResult<JsString> {
    match form {
        "NFC" => Ok(transform_scalar_runs(value, |text| text.nfc().collect())),
        "NFD" => Ok(transform_scalar_runs(value, |text| text.nfd().collect())),
        "NFKC" => Ok(transform_scalar_runs(value, |text| text.nfkc().collect())),
        "NFKD" => Ok(transform_scalar_runs(value, |text| text.nfkd().collect())),
        _ => Err(type_error(
            "String.prototype.normalize form must be NFC, NFD, NFKC, or NFKD",
        )),
    }
}

pub fn is_well_formed(value: &JsString) -> bool {
    char::decode_utf16(value.units().iter().copied()).all(|value| value.is_ok())
}

pub fn to_well_formed_exact(value: &JsString) -> JsString {
    let mut output = Vec::new();
    for decoded in char::decode_utf16(value.units().iter().copied()) {
        let character = decoded.unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut encoded = [0; 2];
        output.extend(character.encode_utf16(&mut encoded).iter().copied());
    }
    JsString::from_units(output)
}

pub fn to_well_formed(value: &JsString) -> String {
    to_well_formed_exact(value)
        .to_utf8()
        .expect("toWellFormed always produces valid Unicode scalar values")
}

pub fn concat(value: &JsString, strings: &[&JsString]) -> JsString {
    let mut parts = Vec::with_capacity(strings.len() + 1);
    parts.push(value);
    parts.extend_from_slice(strings);
    JsString::concat_strs(&parts)
}

pub fn from_char_code(code_units: &[f64]) -> JsString {
    JsString::from_units(
        code_units
            .iter()
            .map(|value| to_uint32(*value) as u16)
            .collect::<Vec<_>>(),
    )
}

pub fn from_code_point(code_points: &[f64]) -> JsResult<JsString> {
    let mut units = Vec::new();
    for value in code_points {
        if !value.is_finite() || value.fract() != 0.0 || *value < 0.0 || *value > 0x10ffff as f64 {
            return Err(range_error(
                "fromCodePoint expects an integer between 0 and 0x10FFFF",
            ));
        }
        let code_point = *value as u32;
        if code_point <= 0xffff {
            units.push(code_point as u16);
        } else {
            let scalar = code_point - 0x1_0000;
            units.push(0xd800 | ((scalar >> 10) as u16));
            units.push(0xdc00 | ((scalar & 0x3ff) as u16));
        }
    }
    Ok(JsString::from_units(units))
}

pub fn raw(raw_parts: &[JsString], substitutions: &[JsString]) -> JsString {
    let mut parts = Vec::with_capacity(raw_parts.len() + substitutions.len());
    for (index, part) in raw_parts.iter().enumerate() {
        parts.push(part.clone());
        if let Some(value) = substitutions.get(index) {
            parts.push(value.clone());
        }
    }
    JsString::concat(&parts)
}

fn find_units(haystack: &[u16], needle: &[u16], start: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(start.min(haystack.len()));
    }
    if start > haystack.len() || needle.len() > haystack.len() {
        return None;
    }
    (start..=haystack.len() - needle.len())
        .find(|index| haystack[*index..*index + needle.len()] == *needle)
}

fn transform_scalar_runs(value: &JsString, transform: impl Fn(&str) -> String) -> JsString {
    let mut output = Vec::new();
    let mut scalar_run = String::new();
    let flush = |run: &mut String, output: &mut Vec<u16>| {
        if !run.is_empty() {
            output.extend(transform(run).encode_utf16());
            run.clear();
        }
    };
    for decoded in char::decode_utf16(value.units().iter().copied()) {
        match decoded {
            Ok(character) => scalar_run.push(character),
            Err(error) => {
                flush(&mut scalar_run, &mut output);
                output.push(error.unpaired_surrogate());
            }
        }
    }
    flush(&mut scalar_run, &mut output);
    JsString::from_units(output)
}

fn previous_code_point(units: &[u16], end: usize) -> (u32, usize) {
    let last = units[end - 1];
    if (0xdc00..=0xdfff).contains(&last) && end >= 2 {
        let first = units[end - 2];
        if (0xd800..=0xdbff).contains(&first) {
            return (
                0x10000 + (((first as u32 - 0xd800) << 10) | (last as u32 - 0xdc00)),
                2,
            );
        }
    }
    (last as u32, 1)
}

fn is_ecmascript_whitespace(value: u32) -> bool {
    matches!(
        value,
        0x0009..=0x000d
            | 0x0020
            | 0x00a0
            | 0x1680
            | 0x2000..=0x200a
            | 0x2028
            | 0x2029
            | 0x202f
            | 0x205f
            | 0x3000
            | 0xfeff
    )
}
