use std::cell::RefCell;
use std::rc::Rc;
use tsonic_rust_runtime::TsonicResult;

use crate::equality::{JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
struct MapEntry<K, V> {
    key: K,
    value: V,
    present: bool,
}

#[derive(Debug)]
struct JsMapState<K, V> {
    entries: Vec<MapEntry<K, V>>,
    size: usize,
}

#[derive(Debug)]
pub struct JsMap<K, V> {
    state: Rc<RefCell<JsMapState<K, V>>>,
}

impl<K, V> Clone for JsMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<K, V> PartialEq for JsMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<K, V> Eq for JsMap<K, V> {}

impl<K, V> JsSameValueZero for JsMap<K, V> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<K, V> JsStrictEqual for JsMap<K, V> {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<K, V> JsMap<K, V> {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(JsMapState {
                entries: Vec::new(),
                size: 0,
            })),
        }
    }

    pub fn from_entries(entries: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: JsSameValueZero,
    {
        let map = Self::new();
        for (key, value) in entries {
            map.set(key, value);
        }
        map
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn len(&self) -> usize {
        self.state.borrow().size
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        let mut state = self.state.borrow_mut();
        for entry in &mut state.entries {
            entry.present = false;
        }
        state.size = 0;
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: JsSameValueZero<Q>,
        V: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .find(|entry| entry.present && entry.key.same_value_zero(key))
            .map(|entry| entry.value.clone())
    }

    pub fn get_eq(&self, key: &K) -> Option<V>
    where
        K: PartialEq,
        V: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .find(|entry| entry.present && entry.key == *key)
            .map(|entry| entry.value.clone())
    }

    pub fn set(&self, key: K, value: V) -> Self
    where
        K: JsSameValueZero,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.key.same_value_zero(&key))
        {
            entry.value = value;
        } else {
            state.entries.push(MapEntry {
                key,
                value,
                present: true,
            });
            state.size += 1;
        }
        drop(state);
        self.clone()
    }

    pub fn set_eq(&self, key: K, value: V) -> Self
    where
        K: PartialEq,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.key == key)
        {
            entry.value = value;
        } else {
            state.entries.push(MapEntry {
                key,
                value,
                present: true,
            });
            state.size += 1;
        }
        drop(state);
        self.clone()
    }

    pub fn has<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: JsSameValueZero<Q>,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.present && entry.key.same_value_zero(key))
    }

    pub fn has_eq(&self, key: &K) -> bool
    where
        K: PartialEq,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.present && entry.key == *key)
    }

    pub fn delete<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: JsSameValueZero<Q>,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.key.same_value_zero(key))
        {
            entry.present = false;
            state.size -= 1;
            return true;
        }
        false
    }

    pub fn delete_eq(&self, key: &K) -> bool
    where
        K: PartialEq,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.key == *key)
        {
            entry.present = false;
            state.size -= 1;
            return true;
        }
        false
    }

    pub fn keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .filter(|entry| entry.present)
            .map(|entry| entry.key.clone())
            .collect()
    }

    pub fn values(&self) -> Vec<V>
    where
        V: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .filter(|entry| entry.present)
            .map(|entry| entry.value.clone())
            .collect()
    }

    pub fn entries(&self) -> Vec<(K, V)>
    where
        K: Clone,
        V: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .filter(|entry| entry.present)
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect()
    }

    pub fn for_each_zero<F>(&self, mut callback: F)
    where
        K: Clone,
        V: Clone,
        F: FnMut(),
    {
        self.for_each(|_, _, _| callback());
    }

    pub fn for_each_value<F>(&self, mut callback: F)
    where
        K: Clone,
        V: Clone,
        F: FnMut(V),
    {
        self.for_each(|value, _, _| callback(value));
    }

    pub fn for_each_value_key<F>(&self, mut callback: F)
    where
        K: Clone,
        V: Clone,
        F: FnMut(V, K),
    {
        self.for_each(|value, key, _| callback(value, key));
    }

    pub fn for_each<F>(&self, mut callback: F)
    where
        K: Clone,
        V: Clone,
        F: FnMut(V, K, Self),
    {
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && !state.entries[index].present {
                    index += 1;
                }
                state
                    .entries
                    .get(index)
                    .map(|entry| (entry.key.clone(), entry.value.clone()))
            };
            let Some((key, value)) = next else {
                break;
            };
            index += 1;
            callback(value, key, self.clone());
        }
    }

    fn try_for_each_with<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        K: Clone,
        V: Clone,
        F: FnMut(V, K, Self) -> TsonicResult<()>,
    {
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && !state.entries[index].present {
                    index += 1;
                }
                state
                    .entries
                    .get(index)
                    .map(|entry| (entry.key.clone(), entry.value.clone()))
            };
            let Some((key, value)) = next else {
                break;
            };
            index += 1;
            callback(value, key, self.clone())?;
        }
        Ok(())
    }

    pub fn try_for_each_zero<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        K: Clone,
        V: Clone,
        F: FnMut() -> TsonicResult<()>,
    {
        self.try_for_each_with(|_, _, _| callback())
    }

    pub fn try_for_each_value<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        K: Clone,
        V: Clone,
        F: FnMut(V) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, _, _| callback(value))
    }

    pub fn try_for_each_value_key<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        K: Clone,
        V: Clone,
        F: FnMut(V, K) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, key, _| callback(value, key))
    }

    pub fn try_for_each<F>(&self, callback: F) -> TsonicResult<()>
    where
        K: Clone,
        V: Clone,
        F: FnMut(V, K, Self) -> TsonicResult<()>,
    {
        self.try_for_each_with(callback)
    }
}

impl<K, V> Default for JsMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
