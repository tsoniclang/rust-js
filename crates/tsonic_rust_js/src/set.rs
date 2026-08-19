use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tsonic_rust_runtime::TsonicResult;

use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
struct SetEntry<T> {
    value: T,
    hash: u64,
    present: bool,
}

#[derive(Debug)]
struct JsSetState<T> {
    entries: Vec<SetEntry<T>>,
    indices_by_hash: HashMap<u64, Vec<usize>>,
    size: usize,
}

#[derive(Debug)]
pub struct JsSet<T> {
    state: Rc<RefCell<JsSetState<T>>>,
}

impl<T> Clone for JsSet<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<T> PartialEq for JsSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for JsSet<T> {}

impl<T> JsSameValueZero for JsSet<T> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsHash for JsSet<T> {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
    }
}

impl<T> JsStrictEqual for JsSet<T> {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsSet<T> {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(JsSetState {
                entries: Vec::new(),
                indices_by_hash: HashMap::new(),
                size: 0,
            })),
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = T>) -> Self
    where
        T: JsHash + JsSameValueZero,
    {
        let set = Self::new();
        for value in values {
            set.add(value);
        }
        set
    }

    pub fn from_array(values: &crate::array::JsArray<T>) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(values.iter_values())
    }

    pub fn from_fixed_array<const LENGTH: usize>(values: &[T; LENGTH]) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(values.iter().cloned())
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
        state.indices_by_hash.clear();
        state.size = 0;
    }

    pub fn has<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
        Q: JsHash,
    {
        find_index(&self.state.borrow(), value.js_hash(), value).is_some()
    }

    pub fn has_eq(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.present && entry.value == *value)
    }

    pub fn add(&self, value: T) -> Self
    where
        T: JsHash + JsSameValueZero,
    {
        self.add_discard(value);
        self.clone()
    }

    pub fn add_discard(&self, value: T)
    where
        T: JsHash + JsSameValueZero,
    {
        let hash = value.js_hash();
        let mut state = self.state.borrow_mut();
        if find_index(&state, hash, &value).is_none() {
            let index = state.entries.len();
            state.entries.push(SetEntry {
                value,
                hash,
                present: true,
            });
            state.indices_by_hash.entry(hash).or_default().push(index);
            state.size += 1;
        }
    }

    pub fn add_eq(&self, value: T) -> Self
    where
        T: PartialEq,
    {
        self.add_eq_discard(value);
        self.clone()
    }

    pub fn add_eq_discard(&self, value: T)
    where
        T: PartialEq,
    {
        if !self.has_eq(&value) {
            let mut state = self.state.borrow_mut();
            state.entries.push(SetEntry {
                value,
                hash: 0,
                present: true,
            });
            state.size += 1;
        }
    }

    pub fn delete<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
        Q: JsHash,
    {
        let hash = value.js_hash();
        let mut state = self.state.borrow_mut();
        if let Some(index) = find_index(&state, hash, value) {
            state.entries[index].present = false;
            remove_hash_index(&mut state.indices_by_hash, hash, index);
            state.size -= 1;
            return true;
        }
        false
    }

    pub fn delete_eq(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.value == *value)
        {
            entry.present = false;
            state.size -= 1;
            return true;
        }
        false
    }

    pub fn keys(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.values()
    }

    pub fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .filter(|entry| entry.present)
            .map(|entry| entry.value.clone())
            .collect()
    }

    pub fn entries(&self) -> Vec<(T, T)>
    where
        T: Clone,
    {
        self.values()
            .into_iter()
            .map(|value| (value.clone(), value))
            .collect()
    }

    pub fn for_each_zero<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(),
    {
        self.for_each(|_, _, _| callback());
    }

    pub fn for_each_value<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T),
    {
        self.for_each(|value, _, _| callback(value));
    }

    pub fn for_each_value_key<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, T),
    {
        self.for_each(|value, key, _| callback(value, key));
    }

    pub fn for_each<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, T, Self),
    {
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && !state.entries[index].present {
                    index += 1;
                }
                state.entries.get(index).map(|entry| entry.value.clone())
            };
            let Some(value) = next else {
                break;
            };
            index += 1;
            callback(value.clone(), value, self.clone());
        }
    }

    pub fn difference(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| !other.has(value)))
    }

    pub fn intersection(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| other.has(value)))
    }

    pub fn union(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        let output = Self::from_values(self.values());
        for value in other.values() {
            output.add(value);
        }
        output
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        let output = self.difference(other);
        for value in other.values() {
            if !self.has(&value) {
                output.add(value);
            }
        }
        output
    }

    pub fn is_subset_of(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        self.values().iter().all(|value| other.has(value))
    }

    pub fn is_superset_of(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        other.is_subset_of(self)
    }

    pub fn is_disjoint_from(&self, other: &Self) -> bool
    where
        T: Clone + JsHash + JsSameValueZero,
    {
        self.values().iter().all(|value| !other.has(value))
    }

    fn try_for_each_with<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T, Self) -> TsonicResult<()>,
    {
        let mut index = 0;
        loop {
            let next = {
                let state = self.state.borrow();
                while index < state.entries.len() && !state.entries[index].present {
                    index += 1;
                }
                state.entries.get(index).map(|entry| entry.value.clone())
            };
            let Some(value) = next else {
                break;
            };
            index += 1;
            callback(value.clone(), value, self.clone())?;
        }
        Ok(())
    }

    pub fn try_for_each_zero<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut() -> TsonicResult<()>,
    {
        self.try_for_each_with(|_, _, _| callback())
    }

    pub fn try_for_each_value<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, _, _| callback(value))
    }

    pub fn try_for_each_value_key<F>(&self, mut callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T) -> TsonicResult<()>,
    {
        self.try_for_each_with(|value, key, _| callback(value, key))
    }

    pub fn try_for_each<F>(&self, callback: F) -> TsonicResult<()>
    where
        T: Clone,
        F: FnMut(T, T, Self) -> TsonicResult<()>,
    {
        self.try_for_each_with(callback)
    }
}

fn find_index<T, Q: ?Sized>(state: &JsSetState<T>, hash: u64, value: &Q) -> Option<usize>
where
    T: JsSameValueZero<Q>,
{
    state
        .indices_by_hash
        .get(&hash)?
        .iter()
        .copied()
        .find(|index| {
            let entry = &state.entries[*index];
            entry.present && entry.hash == hash && entry.value.same_value_zero(value)
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

impl<T> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
