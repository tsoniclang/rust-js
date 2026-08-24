//! Closed own-property object carrier.

use std::collections::HashMap;
use std::fmt;

use crate::equality::JsSameValue;
use crate::errors::{type_error, JsResult};
use crate::value::JsValue;
use crate::JsString;

pub type JsPropertyValue = JsValue;

pub fn is(values: [JsValue; 2]) -> bool {
    values[0].same_value(&values[1])
}

#[derive(Debug, Clone, PartialEq)]
struct ObjectEntry {
    key: JsString,
    value: JsPropertyValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsObject {
    entries: Vec<ObjectEntry>,
    indexes: HashMap<JsString, usize>,
}

impl JsObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_pairs<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: AsRef<str>,
        V: Into<JsPropertyValue>,
    {
        let mut object = Self::new();
        for (key, value) in pairs {
            object.set(key.as_ref(), value);
        }
        object
    }

    pub fn from_exact_pairs<V>(pairs: impl IntoIterator<Item = (JsString, V)>) -> Self
    where
        V: Into<JsPropertyValue>,
    {
        let mut object = Self::new();
        for (key, value) in pairs {
            object.set_exact(key, value);
        }
        object
    }

    pub fn get(&self, key: &str) -> JsValue {
        self.get_ref(key).cloned().unwrap_or(JsValue::Undefined)
    }

    pub fn get_ref(&self, key: &str) -> Option<&JsValue> {
        self.get_exact_ref(&JsString::from_utf8(key))
    }

    pub fn get_exact(&self, key: &JsString) -> JsValue {
        self.get_exact_ref(key)
            .cloned()
            .unwrap_or(JsValue::Undefined)
    }

    pub fn get_exact_ref(&self, key: &JsString) -> Option<&JsValue> {
        self.indexes
            .get(key)
            .and_then(|index| self.entries.get(*index))
            .map(|entry| &entry.value)
    }

    pub fn set(&mut self, key: &str, value: impl Into<JsPropertyValue>) {
        self.set_exact(JsString::from_utf8(key), value);
    }

    pub fn set_exact(&mut self, key: JsString, value: impl Into<JsPropertyValue>) {
        let value = value.into();
        match self.indexes.get(&key).copied() {
            Some(index) => self.entries[index].value = value,
            None => {
                let index = self.entries.len();
                self.indexes.insert(key.clone(), index);
                self.entries.push(ObjectEntry { key, value });
            }
        }
    }

    pub fn delete(&mut self, key: &str) -> bool {
        self.delete_exact(&JsString::from_utf8(key))
    }

    pub fn delete_exact(&mut self, key: &JsString) -> bool {
        if let Some(index) = self.indexes.remove(key) {
            self.entries.remove(index);
            for current in self.indexes.values_mut() {
                if *current > index {
                    *current -= 1;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn has_own_property(&self, key: &str) -> bool {
        self.has_exact_own_property(&JsString::from_utf8(key))
    }

    pub fn has_exact_own_property(&self, key: &JsString) -> bool {
        self.indexes.contains_key(key)
    }

    pub fn keys(&self) -> JsResult<Vec<String>> {
        self.keys_exact().iter().map(exact_key_to_native).collect()
    }

    pub fn keys_exact(&self) -> Vec<JsString> {
        self.ordered_entries()
            .map(|entry| entry.key.clone())
            .collect()
    }

    pub fn values(&self) -> Vec<JsPropertyValue> {
        self.ordered_entries()
            .map(|entry| entry.value.clone())
            .collect()
    }

    pub fn entries(&self) -> JsResult<Vec<(String, JsPropertyValue)>> {
        self.entries_exact()
            .into_iter()
            .map(|(key, value)| Ok((exact_key_to_native(&key)?, value)))
            .collect()
    }

    pub fn entries_exact(&self) -> Vec<(JsString, JsPropertyValue)> {
        self.ordered_entries()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect()
    }

    pub fn assign(&mut self, sources: &[JsObject]) {
        for source in sources {
            for entry in source.ordered_entries() {
                self.set_exact(entry.key.clone(), entry.value.clone());
            }
        }
    }

    pub fn inspect(&self) -> String {
        let body = self
            .ordered_entries()
            .map(|entry| format!("{}: {}", entry.key.to_utf8_escaped(), entry.value.inspect()))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{{body}}}")
    }

    fn ordered_entries(&self) -> impl Iterator<Item = &ObjectEntry> {
        let mut indexes = (0..self.entries.len()).collect::<Vec<_>>();
        indexes.sort_by(|left, right| {
            match (
                array_index(&self.entries[*left].key),
                array_index(&self.entries[*right].key),
            ) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => left.cmp(right),
            }
        });
        indexes.into_iter().map(|index| &self.entries[index])
    }
}

fn exact_key_to_native(value: &JsString) -> JsResult<String> {
    value.to_utf8().map_err(|_| {
        type_error("JavaScript object key cannot be represented by the native Rust string carrier")
    })
}

fn array_index(key: &JsString) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.code_unit_at(0) == Some(u16::from(b'0'))) {
        return None;
    }
    let mut value = 0_u32;
    for unit in key.units() {
        if !(u16::from(b'0')..=u16::from(b'9')).contains(unit) {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(u32::from(*unit - u16::from(b'0')))?;
    }
    (value != u32::MAX).then_some(value)
}

impl fmt::Display for JsObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inspect())
    }
}
