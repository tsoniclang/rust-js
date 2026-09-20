use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier, TsonicResult};

use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
struct MapEntry<K, V> {
    key: K,
    value: V,
    hash: u64,
}

#[derive(Debug)]
struct JsMapState<K, V> {
    entries: Vec<Option<MapEntry<K, V>>>,
    active_iterators: usize,
    indices_by_hash: HashMap<u64, Vec<usize>>,
    size: usize,
}

impl<K, V> JsMapState<K, V> {
    fn compact(&mut self) {
        if self.active_iterators != 0
            || self.entries.len().saturating_sub(self.size) <= self.size.max(32)
        {
            return;
        }
        self.entries.retain(Option::is_some);
        self.indices_by_hash.clear();
        for (index, entry) in self.entries.iter().enumerate() {
            let entry = entry.as_ref().expect("compacted map entry");
            self.indices_by_hash
                .entry(entry.hash)
                .or_default()
                .push(index);
        }
    }
}

struct MapIteration<'a, K, V>(&'a RefCell<JsMapState<K, V>>);

impl<'a, K, V> MapIteration<'a, K, V> {
    fn new(state: &'a RefCell<JsMapState<K, V>>) -> Self {
        state.borrow_mut().active_iterators += 1;
        Self(state)
    }
}

impl<K, V> Drop for MapIteration<'_, K, V> {
    fn drop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.active_iterators -= 1;
        state.compact();
    }
}

#[derive(Debug)]
pub struct JsMap<K, V> {
    state: Rc<RefCell<JsMapState<K, V>>>,
    identity: ObjectIdentity,
}

impl<K, V> Clone for JsMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            identity: self.identity.clone(),
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

impl<K, V> JsHash for JsMap<K, V> {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
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
                active_iterators: 0,
                indices_by_hash: HashMap::new(),
                size: 0,
            })),
            identity: ObjectIdentity::new(),
        }
    }

    pub fn from_entries(entries: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: JsHash + JsSameValueZero,
    {
        let map = Self::new();
        for (key, value) in entries {
            map.set(key, value);
        }
        map
    }

    pub fn from_array(entries: &crate::array::JsArray<(K, V)>) -> Self
    where
        K: Clone + JsHash + JsSameValueZero,
        V: Clone,
    {
        Self::from_entries(entries.iter_values())
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
        if state.active_iterators == 0 {
            state.entries.clear();
        } else {
            for entry in &mut state.entries {
                *entry = None;
            }
        }
        state.indices_by_hash.clear();
        state.size = 0;
    }

    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: JsSameValueZero<Q>,
        Q: JsHash + ?Sized,
        V: Clone,
    {
        let state = self.state.borrow();
        find_index(&state, key.js_hash(), key).map(|index| {
            state.entries[index]
                .as_ref()
                .expect("live map entry")
                .value
                .clone()
        })
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
            .flatten()
            .find(|entry| entry.key == *key)
            .map(|entry| entry.value.clone())
    }

    pub fn set(&self, key: K, value: V) -> Self
    where
        K: JsHash + JsSameValueZero,
    {
        self.set_discard(key, value);
        self.clone()
    }

    pub fn set_discard(&self, key: K, value: V)
    where
        K: JsHash + JsSameValueZero,
    {
        let hash = key.js_hash();
        let mut state = self.state.borrow_mut();
        if let Some(index) = find_index(&state, hash, &key) {
            state.entries[index].as_mut().expect("live map entry").value = value;
        } else {
            let index = state.entries.len();
            state.entries.push(Some(MapEntry { key, value, hash }));
            state.indices_by_hash.entry(hash).or_default().push(index);
            state.size += 1;
        }
    }

    pub fn set_eq(&self, key: K, value: V) -> Self
    where
        K: PartialEq,
    {
        self.set_eq_discard(key, value);
        self.clone()
    }

    pub fn set_eq_discard(&self, key: K, value: V)
    where
        K: PartialEq,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.key == key)
        {
            entry.value = value;
        } else {
            state.entries.push(Some(MapEntry {
                key,
                value,
                hash: 0,
            }));
            state.size += 1;
        }
    }

    pub fn has<Q>(&self, key: &Q) -> bool
    where
        K: JsSameValueZero<Q>,
        Q: JsHash + ?Sized,
    {
        find_index(&self.state.borrow(), key.js_hash(), key).is_some()
    }

    pub fn has_eq(&self, key: &K) -> bool
    where
        K: PartialEq,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .flatten()
            .any(|entry| entry.key == *key)
    }

    pub fn delete<Q>(&self, key: &Q) -> bool
    where
        K: JsSameValueZero<Q>,
        Q: JsHash + ?Sized,
    {
        let hash = key.js_hash();
        let mut state = self.state.borrow_mut();
        if let Some(index) = find_index(&state, hash, key) {
            state.entries[index] = None;
            remove_hash_index(&mut state.indices_by_hash, hash, index);
            state.size -= 1;
            state.compact();
            return true;
        }
        false
    }

    pub fn delete_eq(&self, key: &K) -> bool
    where
        K: PartialEq,
    {
        let mut state = self.state.borrow_mut();
        if let Some(index) = state
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.key == *key))
        {
            let hash = state.entries[index].as_ref().expect("live map entry").hash;
            state.entries[index] = None;
            remove_hash_index(&mut state.indices_by_hash, hash, index);
            state.size -= 1;
            state.compact();
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
            .flatten()
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
            .flatten()
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
            .flatten()
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
        let _iteration = MapIteration::new(&self.state);
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && state.entries[index].is_none() {
                    index += 1;
                }
                state
                    .entries
                    .get(index)
                    .and_then(Option::as_ref)
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
        let _iteration = MapIteration::new(&self.state);
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && state.entries[index].is_none() {
                    index += 1;
                }
                state
                    .entries
                    .get(index)
                    .and_then(Option::as_ref)
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

fn find_index<K, V, Q: ?Sized>(state: &JsMapState<K, V>, hash: u64, key: &Q) -> Option<usize>
where
    K: JsSameValueZero<Q>,
{
    state
        .indices_by_hash
        .get(&hash)?
        .iter()
        .copied()
        .find(|index| {
            state.entries[*index]
                .as_ref()
                .is_some_and(|entry| entry.hash == hash && entry.key.same_value_zero(key))
        })
}

fn remove_hash_index(indices_by_hash: &mut HashMap<u64, Vec<usize>>, hash: u64, index: usize) {
    let remove_bucket = if let Some(indices) = indices_by_hash.get_mut(&hash) {
        indices.retain(|candidate| *candidate != index);
        indices.is_empty()
    } else {
        false
    };
    if remove_bucket {
        indices_by_hash.remove(&hash);
    }
}

impl<K, V> ObjectIdentityCarrier for JsMap<K, V> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

impl<K, V> Default for JsMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::JsMap;

    #[test]
    fn identity_key_churn_reclaims_tombstones() {
        let values = JsMap::new();
        for index in 0..4096 {
            values.set_eq_discard(index, index);
            assert!(values.delete_eq(&index));
            assert!(values.state.borrow().entries.len() <= 32);
        }
        assert!(values.is_empty());
        values.set_eq_discard(1, 7);
        assert_eq!(values.get_eq(&1), Some(7));
    }
}
