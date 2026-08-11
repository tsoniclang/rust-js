use std::cell::RefCell;
use std::rc::Rc;

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

    pub fn has(&self, value: &T) -> bool
    where
        T: JsSameValueZero,
    {
        self.state
            .borrow()
            .entries
            .iter()
            .any(|entry| entry.present && entry.value.same_value_zero(value))
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

    pub fn delete(&self, value: &T) -> bool
    where
        T: JsSameValueZero,
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

    pub fn for_each<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(&T, &T, &Self),
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
            callback(&value, &value, self);
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
}

impl<T> Default for JsSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
