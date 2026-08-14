use std::cell::RefCell;
use std::rc::Rc;
use tsonic_rust_runtime::TsonicResult;

use crate::equality::{JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
struct SetEntry<T> {
    value: T,
    present: bool,
}

#[derive(Debug)]
struct JsSetState<T> {
    entries: Vec<SetEntry<T>>,
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
                size: 0,
            })),
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = T>) -> Self
    where
        T: JsSameValueZero,
    {
        let set = Self::new();
        for value in values {
            set.add(value);
        }
        set
    }

    pub fn from_array(values: &crate::array::JsArray<T>) -> Self
    where
        T: Clone + JsSameValueZero,
    {
        Self::from_values(values.iter_values())
    }

    pub fn from_fixed_array<const LENGTH: usize>(values: &[T; LENGTH]) -> Self
    where
        T: Clone + JsSameValueZero,
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
        state.size = 0;
    }

    pub fn has<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.present && entry.value.same_value_zero(value))
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
        T: JsSameValueZero,
    {
        if !self.has(&value) {
            let mut state = self.state.borrow_mut();
            state.entries.push(SetEntry {
                value,
                present: true,
            });
            state.size += 1;
        }
        self.clone()
    }

    pub fn add_eq(&self, value: T) -> Self
    where
        T: PartialEq,
    {
        if !self.has_eq(&value) {
            let mut state = self.state.borrow_mut();
            state.entries.push(SetEntry {
                value,
                present: true,
            });
            state.size += 1;
        }
        self.clone()
    }

    pub fn delete<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: JsSameValueZero<Q>,
    {
        let mut state = self.state.borrow_mut();
        if let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.present && entry.value.same_value_zero(value))
        {
            entry.present = false;
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
        T: Clone + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| !other.has(value)))
    }

    pub fn intersection(&self, other: &Self) -> Self
    where
        T: Clone + JsSameValueZero,
    {
        Self::from_values(self.values().into_iter().filter(|value| other.has(value)))
    }

    pub fn union(&self, other: &Self) -> Self
    where
        T: Clone + JsSameValueZero,
    {
        let output = Self::from_values(self.values());
        for value in other.values() {
            output.add(value);
        }
        output
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self
    where
        T: Clone + JsSameValueZero,
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
        T: Clone + JsSameValueZero,
    {
        self.values().iter().all(|value| other.has(value))
    }

    pub fn is_superset_of(&self, other: &Self) -> bool
    where
        T: Clone + JsSameValueZero,
    {
        other.is_subset_of(self)
    }

    pub fn is_disjoint_from(&self, other: &Self) -> bool
    where
        T: Clone + JsSameValueZero,
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

impl<T> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
