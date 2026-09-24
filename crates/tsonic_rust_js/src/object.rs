//! Closed own-property object carrier.

use hashbrown::HashMap;
use std::fmt;

use crate::equality::JsSameValue;
use crate::errors::JsResult;
use crate::value::JsValue;
use crate::JsString;

mod identity;
mod key;
pub(crate) use key::PropertyKey;

pub type JsPropertyValue = JsValue;

pub fn is(values: [JsValue; 2]) -> bool {
    values[0].same_value(&values[1])
}

#[derive(Debug, Clone, PartialEq)]
struct ObjectEntry {
    key: PropertyKey,
    value: JsPropertyValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsObject {
    entries: Vec<ObjectEntry>,
    indexes: HashMap<PropertyKey, usize>,
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
        self.get_ref(key).cloned().unwrap_or(JsValue::Null)
    }

    pub fn get_ref(&self, key: &str) -> Option<&JsValue> {
        self.indexes
            .get(key)
            .map(|index| &self.entries[*index].value)
    }

    pub fn get_exact(&self, key: &JsString) -> JsValue {
        self.get_exact_ref(key).cloned().unwrap_or(JsValue::Null)
    }

    pub fn get_exact_ref(&self, key: &JsString) -> Option<&JsValue> {
        self.indexes
            .get(&PropertyKey::exact(key.clone()))
            .and_then(|index| self.entries.get(*index))
            .map(|entry| &entry.value)
    }

    pub fn set(&mut self, key: &str, value: impl Into<JsPropertyValue>) {
        if let Some(index) = self.indexes.get(key).copied() {
            self.entries[index].value = value.into();
        } else {
            self.set_key(PropertyKey::Native(key.to_owned()), value.into());
        }
    }

    pub fn set_exact(&mut self, key: JsString, value: impl Into<JsPropertyValue>) {
        self.set_key(PropertyKey::exact(key), value.into());
    }

    fn set_key(&mut self, key: PropertyKey, value: JsPropertyValue) {
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
        let removed = self.indexes.remove(key);
        self.remove_index(removed)
    }

    pub fn delete_exact(&mut self, key: &JsString) -> bool {
        let removed = self.indexes.remove(&PropertyKey::exact(key.clone()));
        self.remove_index(removed)
    }

    fn remove_index(&mut self, removed: Option<usize>) -> bool {
        if let Some(index) = removed {
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
        self.indexes.contains_key(key)
    }

    pub fn has_exact_own_property(&self, key: &JsString) -> bool {
        self.indexes.contains_key(&PropertyKey::exact(key.clone()))
    }

    pub fn keys(&self) -> JsResult<Vec<String>> {
        self.ordered_entries()
            .map(|entry| entry.key.to_native())
            .collect()
    }

    pub fn keys_exact(&self) -> Vec<JsString> {
        self.ordered_entries()
            .map(|entry| entry.key.to_exact())
            .collect()
    }

    pub fn values(&self) -> Vec<JsPropertyValue> {
        self.ordered_entries()
            .map(|entry| entry.value.clone())
            .collect()
    }

    pub fn entries(&self) -> JsResult<Vec<(String, JsPropertyValue)>> {
        self.ordered_entries()
            .map(|entry| Ok((entry.key.to_native()?, entry.value.clone())))
            .collect()
    }

    pub fn entries_exact(&self) -> Vec<(JsString, JsPropertyValue)> {
        self.ordered_entries()
            .map(|entry| (entry.key.to_exact(), entry.value.clone()))
            .collect()
    }

    pub(crate) fn serialization_keys(&self) -> Vec<PropertyKey> {
        self.ordered_entries()
            .map(|entry| entry.key.clone())
            .collect()
    }

    pub(crate) fn get_key_ref(&self, key: &PropertyKey) -> Option<&JsValue> {
        self.indexes
            .get(key)
            .map(|index| &self.entries[*index].value)
    }

    pub fn assign(&mut self, sources: &[JsObject]) {
        for source in sources {
            for entry in source.ordered_entries() {
                self.set_key(entry.key.clone(), entry.value.clone());
            }
        }
    }

    pub fn inspect(&self) -> String {
        let body = self
            .ordered_entries()
            .map(|entry| {
                format!(
                    "{}: {}",
                    match &entry.key {
                        PropertyKey::Native(value) => value.clone(),
                        PropertyKey::Utf16(value) => value.to_utf8_escaped(),
                    },
                    entry.value.inspect()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{{body}}}")
    }

    fn ordered_entries(&self) -> impl Iterator<Item = &ObjectEntry> {
        let mut indexes = (0..self.entries.len()).collect::<Vec<_>>();
        indexes.sort_by(|left, right| {
            match (
                self.entries[*left].key.array_index(),
                self.entries[*right].key.array_index(),
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

impl fmt::Display for JsObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inspect())
    }
}
