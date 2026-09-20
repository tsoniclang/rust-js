use std::hash::{Hash, Hasher};

use hashbrown::Equivalent;

use crate::errors::{type_error, JsResult};
use crate::JsString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PropertyKey {
    Native(String),
    Utf16(JsString),
}

impl PropertyKey {
    pub(crate) fn exact(value: JsString) -> Self {
        match value.to_utf8() {
            Ok(text) => Self::Native(text),
            Err(_) => Self::Utf16(value),
        }
    }

    pub(crate) fn to_native(&self) -> JsResult<String> {
        match self {
            Self::Native(value) => Ok(value.clone()),
            Self::Utf16(_) => Err(type_error(
                "Object key cannot be represented by a native Rust string",
            )),
        }
    }

    pub(crate) fn to_exact(&self) -> JsString {
        match self {
            Self::Native(value) => JsString::from_utf8(value),
            Self::Utf16(value) => value.clone(),
        }
    }

    pub(crate) fn array_index(&self) -> Option<u32> {
        let Self::Native(text) = self else {
            return None;
        };
        if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
            return None;
        }
        let mut value = 0_u32;
        for byte in text.bytes() {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
        }
        (value != u32::MAX).then_some(value)
    }
}

impl Hash for PropertyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Native(value) => value.hash(state),
            Self::Utf16(value) => value.hash(state),
        }
    }
}

impl Equivalent<PropertyKey> for str {
    fn equivalent(&self, key: &PropertyKey) -> bool {
        matches!(key, PropertyKey::Native(value) if self == value)
    }
}
