//! Native UTF-8 string operations exposed by the source profile.

use tsonic_rust_runtime::conversions::IntegerInput;
use tsonic_rust_runtime::{JsError, JsErrorKind};
use unicode_normalization::UnicodeNormalization;

use crate::array::JsArray;
use crate::errors::{type_error, JsResult};
use crate::native_integer::{absolute_index, pad_length, relative_index, Integer32};
use crate::number::JsNumberValue;
use crate::numeric::IndexInput;
use crate::JsValue;

pub struct NativeStringIterator {
    value: String,
    offset: usize,
}

impl NativeStringIterator {
    pub fn new(value: String) -> Self {
        Self { value, offset: 0 }
    }
}

impl Iterator for NativeStringIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let character = self.value[self.offset..].chars().next()?;
        self.offset += character.len_utf8();
        Some(character.to_string())
    }
}

impl std::iter::FusedIterator for NativeStringIterator {}

/// JS-facing string value conversion contract used by dense array join and future array helpers.
pub trait JsToString {
    fn to_js_string(&self) -> String;

    fn write_js_string(&self, output: &mut String) {
        output.push_str(&self.to_js_string());
    }

    fn join_js_strings(values: &[Self], separator: &str) -> String
    where
        Self: Sized,
    {
        let mut output = String::new();
        for (index, value) in values.iter().enumerate() {
            if index != 0 {
                output.push_str(separator);
            }
            value.write_js_string(&mut output);
        }
        output
    }
}

macro_rules! impl_js_to_string {
    ($($type:ty),+ $(,)?) => {
        $(impl JsToString for $type {
            fn to_js_string(&self) -> String {
                self.to_string()
            }

            fn write_js_string(&self, output: &mut String) {
                use std::fmt::Write;
                write!(output, "{self}").expect("formatting into String cannot fail");
            }
        })+
    };
}

impl_js_to_string!(bool, i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

macro_rules! impl_source_string_value {
    ($($value:ty),+ $(,)?) => {$(
        impl JsToString for $value {
            fn to_js_string(&self) -> String {
                tsonic_rust_runtime::source_string(self)
            }
        }
    )+};
}

impl_source_string_value!(
    (),
    tsonic_rust_runtime::Null,
    tsonic_rust_runtime::Undefined,
    tsonic_rust_runtime::BigInt,
    tsonic_rust_runtime::TsonicError
);

pub fn from_value<Value: JsToString + ?Sized>(value: &Value) -> String {
    value.to_js_string()
}

impl JsToString for String {
    fn to_js_string(&self) -> String {
        self.clone()
    }

    fn write_js_string(&self, output: &mut String) {
        output.push_str(self);
    }

    fn join_js_strings(values: &[Self], separator: &str) -> String {
        values.join(separator)
    }
}

impl JsToString for f32 {
    fn to_js_string(&self) -> String {
        self.to_js_decimal_string()
    }

    fn write_js_string(&self, output: &mut String) {
        output.push_str(ryu_js::Buffer::new().format(*self));
    }
}

impl JsToString for f64 {
    fn to_js_string(&self) -> String {
        self.to_js_decimal_string()
    }

    fn write_js_string(&self, output: &mut String) {
        output.push_str(ryu_js::Buffer::new().format(*self));
    }
}

impl JsToString for str {
    fn to_js_string(&self) -> String {
        self.to_string()
    }

    fn write_js_string(&self, output: &mut String) {
        output.push_str(self);
    }
}

fn native_slice(value: &str, start: usize, end: usize) -> Result<&str, JsError> {
    value.get(start..end).ok_or_else(|| {
        crate::errors::range_error("string range must start and end at UTF-8 character boundaries")
    })
}

pub fn js_len(value: &str) -> usize {
    value.len()
}

pub fn char_at(value: &str, index: impl IndexInput) -> Result<String, JsError> {
    match absolute_index(index, value.len()) {
        Some(position) => Ok(native_slice(value, position, value.len())?
            .chars()
            .next()
            .map(|character| character.to_string())
            .unwrap_or_default()),
        None => Ok(String::new()),
    }
}

pub fn at(value: &str, index: impl IndexInput) -> Result<Option<String>, JsError> {
    relative_index(index, value.len())
        .map(|position| char_at(value, position))
        .transpose()
}

pub fn char_code_at(value: &str, index: impl IndexInput) -> f64 {
    code_point_at(value, index)
        .map(f64::from)
        .unwrap_or(f64::NAN)
}

pub fn code_point_at(value: &str, index: impl IndexInput) -> Option<u32> {
    let position = absolute_index(index, value.len())?;
    let character = value.get(position..)?.chars().next()?;
    Some(u32::from(character))
}

pub fn slice(
    value: &str,
    start: impl IndexInput,
    end: Option<impl IndexInput>,
) -> Result<String, JsError> {
    let from = crate::native_integer::normalize_slice_index(start, value.len());
    let to = end
        .map(|position| crate::native_integer::normalize_slice_index(position, value.len()))
        .unwrap_or(value.len());
    if from > to {
        return Ok(String::new());
    }
    native_slice(value, from, to).map(str::to_owned)
}

pub fn slice_to(
    value: &str,
    start: impl IndexInput,
    end: impl IndexInput,
) -> Result<String, JsError> {
    slice(value, start, Some(end))
}

pub fn slice_from(value: &str, start: impl IndexInput) -> Result<String, JsError> {
    slice(value, start, None::<usize>)
}

fn substring_with_end(
    value: &str,
    start: impl IndexInput,
    end: Option<impl IndexInput>,
) -> Result<String, JsError> {
    let mut start = clamped_position(start, value.len());
    let mut end = end
        .map(|position| clamped_position(position, value.len()))
        .unwrap_or(value.len());
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    native_slice(value, start, end).map(str::to_owned)
}

pub fn substring(
    value: &str,
    start: impl IndexInput,
    end: impl IndexInput,
) -> Result<String, JsError> {
    substring_with_end(value, start, Some(end))
}

fn substr_with_length(
    value: &str,
    start: impl IndexInput,
    length: Option<impl IndexInput>,
) -> Result<String, JsError> {
    let from = crate::native_integer::normalize_slice_index(start, value.len());
    let to = length
        .map(|length| {
            from.saturating_add(length.positive_index(usize::MAX))
                .min(value.len())
        })
        .unwrap_or(value.len());
    native_slice(value, from, to).map(str::to_owned)
}

pub fn index_of(value: &str, search: &str, position: impl IndexInput) -> isize {
    let mut position = clamped_position(position, value.len());
    if search.is_empty() {
        return position as isize;
    }
    while !value.is_char_boundary(position) {
        position += 1;
    }
    value[position..]
        .find(search)
        .map(|offset| (position + offset) as isize)
        .unwrap_or(-1)
}

fn last_index_of_with_position(
    value: &str,
    search: &str,
    position: Option<impl IndexInput>,
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
    let mut end = position.saturating_add(search.len()).min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end]
        .rfind(search)
        .map(|offset| offset as isize)
        .unwrap_or(-1)
}

pub fn starts_with(value: &str, search: &str, position: impl IndexInput) -> bool {
    let start = clamped_position(position, value.len());
    value
        .get(start..)
        .is_some_and(|suffix| suffix.starts_with(search))
}

pub fn last_index_of(value: &str, search: &str, position: impl IndexInput) -> isize {
    last_index_of_with_position(value, search, Some(position))
}

fn ends_with_position(value: &str, search: &str, end_position: Option<impl IndexInput>) -> bool {
    let end = end_position
        .map(|end| clamped_position(end, value.len()))
        .unwrap_or(value.len());
    value
        .get(..end)
        .is_some_and(|prefix| prefix.ends_with(search))
}

pub fn includes(value: &str, search: &str, position: impl IndexInput) -> bool {
    index_of(value, search, position) >= 0
}

pub fn includes_from_start(value: &str, search: &str) -> bool {
    value.contains(search)
}

pub fn starts_with_from_start(value: &str, search: &str) -> bool {
    value.starts_with(search)
}

pub fn ends_with_at_end(value: &str, search: &str) -> bool {
    value.ends_with(search)
}

pub fn ends_with(value: &str, search: &str, end_position: impl IndexInput) -> bool {
    ends_with_position(value, search, Some(end_position))
}

pub fn index_of_from_start(value: &str, search: &str) -> isize {
    index_of(value, search, 0.0)
}

pub fn last_index_of_from_end(value: &str, search: &str) -> isize {
    last_index_of_with_position(value, search, None::<usize>)
}

pub fn substring_from(value: &str, start: impl IndexInput) -> Result<String, JsError> {
    substring_with_end(value, start, None::<usize>)
}

pub fn substr_from(value: &str, start: impl IndexInput) -> Result<String, JsError> {
    substr_with_length(value, start, None::<usize>)
}

pub fn substr(
    value: &str,
    start: impl IndexInput,
    length: impl IndexInput,
) -> Result<String, JsError> {
    substr_with_length(value, start, Some(length))
}

fn clamped_position(value: impl IndexInput, length: usize) -> usize {
    value.positive_index(length)
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

pub fn replace_with<E, F>(value: &str, search: &str, replacer: F) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    replace_matches(value, search, replacer, false)
}

pub fn try_replace_with<E, F>(value: &str, search: &str, replacer: F) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    replace_with(value, search, replacer)
}

pub fn replace_all_with<E, F>(value: &str, search: &str, replacer: F) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    replace_matches(value, search, replacer, true)
}

fn replace_matches<E, F>(value: &str, search: &str, replacer: F, all: bool) -> Result<String, E>
where
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    let mut output = String::with_capacity(value.len());
    let mut consumed = 0;
    for (offset, matched) in value.match_indices(search) {
        output.push_str(&value[consumed..offset]);
        let arguments = JsArray::from_dense(vec![
            JsValue::String((matched).to_owned()),
            JsValue::from(offset),
            JsValue::String((value).to_owned()),
        ]);
        output.push_str(&replacer(arguments)?);
        consumed = offset + matched.len();
        if !all {
            break;
        }
    }
    output.push_str(&value[consumed..]);
    Ok(output)
}

pub fn try_replace_all_with<E, F>(value: &str, search: &str, replacer: F) -> Result<String, E>
where
    E: From<JsError>,
    F: Fn(JsArray<JsValue>) -> Result<String, E>,
{
    replace_all_with(value, search, replacer)
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
    limit: Option<impl Integer32>,
) -> Result<JsArray<String>, JsError> {
    let limit = crate::native_integer::split_limit(limit);
    if limit == 0 {
        return Ok(JsArray::new());
    }
    if separator.is_empty() {
        let parts = value
            .chars()
            .map(|character| character.to_string())
            .take(limit)
            .collect::<Vec<_>>();
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
    split_with_limit(value, separator, None::<usize>)
}

pub fn split(
    value: &str,
    separator: &str,
    limit: impl Integer32,
) -> Result<JsArray<String>, JsError> {
    split_with_limit(value, separator, Some(limit))
}

pub fn repeat(value: &str, count: impl IndexInput) -> Result<String, JsError> {
    let (count, _) = crate::string_capacity::repeat_shape(count, || js_len(value))?;
    if count == 0 {
        return Ok(String::new());
    }
    let length = value
        .len()
        .checked_mul(count)
        .ok_or_else(|| crate::errors::range_error("invalid string length"))?;
    let mut output = String::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| crate::errors::range_error("invalid string length"))?;
    output.push_str(value);
    while output.len() < length {
        let copied = (length - output.len()).min(output.len());
        output.extend_from_within(..copied);
    }
    Ok(output)
}

pub fn pad_start(value: &str, target_length: impl IndexInput) -> Result<String, JsError> {
    pad(value, target_length, None, true)
}

pub fn pad_start_with(
    value: &str,
    target_length: impl IndexInput,
    filler: &str,
) -> Result<String, JsError> {
    pad(value, target_length, Some(filler), true)
}

pub fn pad_end(value: &str, target_length: impl IndexInput) -> Result<String, JsError> {
    pad(value, target_length, None, false)
}

pub fn pad_end_with(
    value: &str,
    target_length: impl IndexInput,
    filler: &str,
) -> Result<String, JsError> {
    pad(value, target_length, Some(filler), false)
}

fn pad(
    value: &str,
    target_length: impl IndexInput,
    filler: Option<&str>,
    at_start: bool,
) -> Result<String, JsError> {
    let target_length = pad_length(target_length);
    if target_length <= value.len() {
        return Ok(value.to_string());
    }
    let filler = filler.unwrap_or(" ");
    if filler.is_empty() {
        return Ok(value.to_string());
    }
    let needed = target_length - value.len();
    let repetitions = needed / filler.len();
    let remainder = native_slice(filler, 0, needed % filler.len())?;
    let mut output = String::new();
    output
        .try_reserve_exact(target_length)
        .map_err(|_| JsError::new(JsErrorKind::RangeError, "invalid string length"))?;
    if !at_start {
        output.push_str(value);
    }
    for _ in 0..repetitions {
        output.push_str(filler);
    }
    output.push_str(remainder);
    if at_start {
        output.push_str(value);
    }
    Ok(output)
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

pub fn normalize(value: &str) -> String {
    value.nfc().collect()
}

pub fn normalize_with_form(value: &str, form: &str) -> JsResult<String> {
    match form {
        "NFC" => Ok(value.nfc().collect()),
        "NFD" => Ok(value.nfd().collect()),
        "NFKC" => Ok(value.nfkc().collect()),
        "NFKD" => Ok(value.nfkd().collect()),
        _ => Err(type_error(
            "String.prototype.normalize form must be NFC, NFD, NFKC, or NFKD",
        )),
    }
}

pub fn is_well_formed(_value: &str) -> bool {
    true
}

pub fn to_well_formed(value: &str) -> String {
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

pub fn from_char_code<Value: IntegerInput<u32>>(code_units: &[Value]) -> Result<String, JsError> {
    from_code_point(code_units)
}

pub fn from_code_point<Value: IntegerInput<u32>>(code_points: &[Value]) -> Result<String, JsError> {
    let mut output = String::new();
    for value in code_points {
        let point = value
            .checked_integer()
            .filter(|point| *point <= 0x10ffff)
            .ok_or_else(|| {
                JsError::new(
                    JsErrorKind::RangeError,
                    "fromCodePoint expects an integer between 0 and 0x10FFFF",
                )
            })?;
        let character = char::from_u32(point).ok_or_else(|| {
            crate::errors::range_error("native string character must be a Unicode scalar")
        })?;
        output.push(character);
    }
    Ok(output)
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
