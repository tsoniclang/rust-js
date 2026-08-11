//! Closed own-property object carrier.

use std::collections::HashMap;
use std::fmt;

use crate::equality::JsSameValue;
use crate::value::JsValue;

pub type JsPropertyValue = JsValue;

pub fn is(values: [JsValue; 2]) -> bool {
    values[0].same_value(&values[1])
}

#[derive(Debug, Clone, PartialEq)]
struct ObjectEntry {
    key: String,
    value: JsPropertyValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsObject {
    entries: Vec<ObjectEntry>,
    indexes: HashMap<String, usize>,
}

impl JsObject {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_pairs<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<JsPropertyValue>,
    {
        let mut object = Self::new();
        for (key, value) in pairs {
            object.set(key, value);
        }
        object
    }

    pub fn get(&self, key: &str) -> JsValue {
        self.get_ref(key).cloned().unwrap_or(JsValue::Undefined)
    }

    pub fn get_ref(&self, key: &str) -> Option<&JsValue> {
        self.indexes
            .get(key)
            .and_then(|index| self.entries.get(*index))
            .map(|entry| &entry.value)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<JsPropertyValue>) {
        let key = key.into();
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
        self.indexes.contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.ordered_entries()
            .map(|entry| entry.key.clone())
            .collect()
    }

    pub fn values(&self) -> Vec<JsPropertyValue> {
        self.ordered_entries()
            .map(|entry| entry.value.clone())
            .collect()
    }

    pub fn entries(&self) -> Vec<(String, JsPropertyValue)> {
        self.ordered_entries()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect()
    }

    pub fn assign(&mut self, sources: &[JsObject]) {
        for source in sources {
            for entry in source.ordered_entries() {
                self.set(entry.key.clone(), entry.value.clone());
            }
        }
    }

    pub fn inspect(&self) -> String {
        let body = self
            .ordered_entries()
            .map(|entry| format!("{}: {}", entry.key, entry.value.inspect()))
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

fn array_index(key: &str) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key.starts_with('0')) {
        return None;
    }
    let value = key.parse::<u32>().ok()?;
    (value != u32::MAX && value.to_string() == key).then_some(value)
}

impl fmt::Display for JsObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inspect())
    }
}
