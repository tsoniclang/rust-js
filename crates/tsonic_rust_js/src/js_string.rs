use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Range};
use std::sync::Arc;

use crate::errors::{type_error, JsResult};

#[derive(Clone)]
pub struct JsString {
    storage: Arc<[u16]>,
    start: usize,
    end: usize,
}

impl JsString {
    pub fn new() -> Self {
        Self::from_units(Vec::new())
    }

    pub fn from_units(units: impl Into<Vec<u16>>) -> Self {
        let storage: Arc<[u16]> = units.into().into();
        let end = storage.len();
        Self {
            storage,
            start: 0,
            end,
        }
    }

    pub fn from_utf8(value: &str) -> Self {
        Self::from_units(value.encode_utf16().collect::<Vec<_>>())
    }

    pub fn units(&self) -> &[u16] {
        &self.storage[self.start..self.end]
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        assert!(range.start <= range.end && range.end <= self.len());
        Self {
            storage: Arc::clone(&self.storage),
            start: self.start + range.start,
            end: self.start + range.end,
        }
    }

    pub fn code_unit_at(&self, index: usize) -> Option<u16> {
        self.units().get(index).copied()
    }

    pub fn code_point_at(&self, index: usize) -> Option<u32> {
        let first = self.code_unit_at(index)?;
        if (0xD800..=0xDBFF).contains(&first) {
            if let Some(second @ 0xDC00..=0xDFFF) = self.code_unit_at(index + 1) {
                return Some(
                    0x10000 + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00)),
                );
            }
        }
        Some(first as u32)
    }

    pub fn advance_index(&self, index: usize, unicode: bool) -> usize {
        if !unicode || index + 1 >= self.len() {
            return index.saturating_add(1);
        }
        match (self.code_unit_at(index), self.code_unit_at(index + 1)) {
            (Some(0xD800..=0xDBFF), Some(0xDC00..=0xDFFF)) => index + 2,
            _ => index + 1,
        }
    }

    pub fn to_utf8(&self) -> Result<String, std::char::DecodeUtf16Error> {
        char::decode_utf16(self.units().iter().copied()).collect()
    }

    pub(crate) fn to_utf8_lossy(&self) -> String {
        char::decode_utf16(self.units().iter().copied())
            .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    pub(crate) fn to_utf8_escaped(&self) -> String {
        let mut output = String::new();
        for decoded in char::decode_utf16(self.units().iter().copied()) {
            match decoded {
                Ok(character) => output.extend(character.escape_debug()),
                Err(error) => {
                    use fmt::Write as _;
                    write!(output, "\\u{:04x}", error.unpaired_surrogate())
                        .expect("writing to a String cannot fail");
                }
            }
        }
        output
    }

    pub(crate) fn inspect_quoted(&self) -> String {
        format!("\"{}\"", self.to_utf8_escaped())
    }

    pub fn concat(parts: &[Self]) -> Self {
        let length = parts.iter().map(Self::len).sum();
        let mut units = Vec::with_capacity(length);
        for part in parts {
            units.extend_from_slice(part.units());
        }
        Self::from_units(units)
    }

    pub fn concat_strs(parts: &[&Self]) -> Self {
        let length = parts.iter().map(|part| part.len()).sum();
        let mut units = Vec::with_capacity(length);
        for part in parts {
            units.extend_from_slice(part.units());
        }
        Self::from_units(units)
    }

    pub fn concat_values<const N: usize>(&self, values: [Self; N]) -> Self {
        let length = self.len() + values.iter().map(Self::len).sum::<usize>();
        let mut units = Vec::with_capacity(length);
        units.extend_from_slice(self.units());
        for value in values {
            units.extend_from_slice(value.units());
        }
        Self::from_units(units)
    }

    pub fn starts_with_at(&self, position: usize, value: &Self) -> bool {
        position <= self.len()
            && value.len() <= self.len() - position
            && self.units()[position..position + value.len()] == *value.units()
    }
}

pub fn from_utf8_string(value: String) -> JsString {
    JsString::from_utf8(&value)
}

pub(crate) fn to_native_string(value: &JsString, context: &str) -> JsResult<String> {
    value.to_utf8().map_err(|_| {
        type_error(format!(
            "{context} cannot be represented by the native Rust string carrier"
        ))
    })
}

impl Default for JsString {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&JsString> for JsString {
    fn from(value: &JsString) -> Self {
        value.clone()
    }
}

impl From<Vec<u16>> for JsString {
    fn from(value: Vec<u16>) -> Self {
        Self::from_units(value)
    }
}

impl PartialEq for JsString {
    fn eq(&self, other: &Self) -> bool {
        self.units() == other.units()
    }
}

impl Eq for JsString {}

impl PartialOrd for JsString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.units().cmp(other.units())
    }
}

impl Hash for JsString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.units().hash(state);
    }
}

impl fmt::Debug for JsString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "JsString({})", self.inspect_quoted())
    }
}

impl Add for JsString {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::concat_strs(&[&self, &rhs])
    }
}

impl Add<&JsString> for JsString {
    type Output = Self;

    fn add(self, rhs: &JsString) -> Self::Output {
        Self::concat_strs(&[&self, rhs])
    }
}

impl Add<JsString> for &JsString {
    type Output = JsString;

    fn add(self, rhs: JsString) -> Self::Output {
        JsString::concat_strs(&[self, &rhs])
    }
}

impl Add<&JsString> for &JsString {
    type Output = JsString;

    fn add(self, rhs: &JsString) -> Self::Output {
        JsString::concat_strs(&[self, rhs])
    }
}
