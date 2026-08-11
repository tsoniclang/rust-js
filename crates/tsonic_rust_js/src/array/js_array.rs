use std::cell::RefCell;
use std::rc::Rc;

use super::slot::JsSlot;
use crate::coercion::relative_index;
use crate::equality::{JsSameValueZero, JsStrictEqual};

#[derive(Debug)]
struct JsArrayState<T> {
    slots: Vec<JsSlot<T>>,
}

#[derive(Debug)]
pub struct JsArray<T> {
    state: Rc<RefCell<JsArrayState<T>>>,
}

pub struct JsArrayIterator<T> {
    array: JsArray<T>,
    index: usize,
}

impl<T> Clone for JsArray<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

impl<T> PartialEq for JsArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for JsArray<T> {}

impl<T> JsSameValueZero for JsArray<T> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsStrictEqual for JsArray<T> {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsArray<T> {
    pub fn new() -> Self {
        Self::from_slots(Vec::new())
    }

    pub fn with_length(length: usize) -> Self {
        let mut slots = Vec::new();
        slots.resize_with(length, || JsSlot::Hole);
        Self::from_slots(slots)
    }

    pub fn from_dense(values: Vec<T>) -> Self {
        Self::from_slots(values.into_iter().map(JsSlot::Present).collect())
    }

    pub fn from_sparse(length: usize, values: Vec<(usize, T)>) -> Self {
        let result = Self::with_length(length);
        for (index, value) in values {
            assert!(
                index < length,
                "sparse array index exceeds its declared length"
            );
            result.set(index, value);
        }
        result
    }

    fn from_slots(slots: Vec<JsSlot<T>>) -> Self {
        Self {
            state: Rc::new(RefCell::new(JsArrayState { slots })),
        }
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.state) as usize
    }

    pub fn len(&self) -> usize {
        self.state.borrow().slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: usize) {
        let mut state = self.state.borrow_mut();
        state.slots.truncate(len);
        state.slots.resize_with(len, || JsSlot::Hole);
    }

    pub fn has_index(&self, index: usize) -> bool {
        matches!(
            self.state.borrow().slots.get(index),
            Some(JsSlot::Present(_))
        )
    }

    pub fn delete_at(&self, index: usize) -> bool {
        let mut state = self.state.borrow_mut();
        if let Some(slot) = state.slots.get_mut(index) {
            *slot = JsSlot::Hole;
        }
        true
    }

    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.state
            .borrow()
            .slots
            .get(index)
            .and_then(JsSlot::as_ref)
            .cloned()
    }

    pub fn at(&self, index: f64) -> Option<T>
    where
        T: Clone,
    {
        self.get(relative_index(index, self.len())?)
    }

    pub fn set(&self, index: usize, value: T) {
        let mut state = self.state.borrow_mut();
        if state.slots.len() <= index {
            state.slots.resize_with(index + 1, || JsSlot::Hole);
        }
        state.slots[index] = JsSlot::Present(value);
    }

    pub fn push(&self, value: T) -> usize {
        let mut state = self.state.borrow_mut();
        state.slots.push(JsSlot::Present(value));
        state.slots.len()
    }

    pub fn pop(&self) -> Option<T> {
        match self.state.borrow_mut().slots.pop() {
            Some(JsSlot::Present(value)) => Some(value),
            _ => None,
        }
    }

    pub fn shift(&self) -> Option<T> {
        let mut state = self.state.borrow_mut();
        if state.slots.is_empty() {
            return None;
        }
        match state.slots.remove(0) {
            JsSlot::Present(value) => Some(value),
            JsSlot::Hole => None,
        }
    }

    pub fn unshift(&self, value: T) -> usize {
        let mut state = self.state.borrow_mut();
        state.slots.insert(0, JsSlot::Present(value));
        state.slots.len()
    }

    pub fn fill(&self, value: T, start: isize, end: Option<isize>)
    where
        T: Clone,
    {
        let (start, end) = normalize_range(self.len(), start, end);
        let mut state = self.state.borrow_mut();
        for slot in &mut state.slots[start..end] {
            *slot = JsSlot::Present(value.clone());
        }
    }

    pub fn copy_within(&self, target: isize, start: isize, end: Option<isize>)
    where
        T: Clone,
    {
        let len = self.len();
        let to = normalize_index(len, target);
        let (from, end) = normalize_range(len, start, end);
        let count = end.saturating_sub(from).min(len.saturating_sub(to));
        let copied = self.state.borrow().slots[from..from + count].to_vec();
        let mut state = self.state.borrow_mut();
        for (offset, slot) in copied.into_iter().enumerate() {
            state.slots[to + offset] = slot;
        }
    }

    pub fn reverse(&self) {
        self.state.borrow_mut().slots.reverse();
    }

    pub fn splice(&self, start: isize, delete_count: usize, items: Vec<T>) -> JsArray<T> {
        let len = self.len();
        let start = normalize_index(len, start);
        let delete_count = delete_count.min(len.saturating_sub(start));
        let removed = self
            .state
            .borrow_mut()
            .slots
            .splice(
                start..start + delete_count,
                items.into_iter().map(JsSlot::Present),
            )
            .collect();
        Self::from_slots(removed)
    }

    pub fn keys(&self) -> Vec<usize> {
        (0..self.len()).collect()
    }

    pub fn enumerable_own_keys(&self) -> Vec<String> {
        self.state
            .borrow()
            .slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| match slot {
                JsSlot::Present(_) => Some(index.to_string()),
                JsSlot::Hole => None,
            })
            .collect()
    }

    pub fn values(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.state
            .borrow()
            .slots
            .iter()
            .map(|slot| slot.as_ref().cloned())
            .collect()
    }

    pub fn entries(&self) -> Vec<(usize, Option<T>)>
    where
        T: Clone,
    {
        self.values().into_iter().enumerate().collect()
    }

    pub fn iter_values(&self) -> JsArrayIterator<T> {
        JsArrayIterator {
            array: self.clone(),
            index: 0,
        }
    }

    pub fn includes(&self, value: &T, from_index: isize) -> bool
    where
        T: JsSameValueZero,
    {
        let state = self.state.borrow();
        if from_index >= state.slots.len() as isize {
            return false;
        }
        let start = normalize_from_index(state.slots.len(), from_index);
        state.slots[start..].iter().any(|slot| {
            slot.as_ref()
                .is_some_and(|item| item.same_value_zero(value))
        })
    }

    pub fn includes_from_start(&self, value: &T) -> bool
    where
        T: JsSameValueZero,
    {
        self.includes(value, 0)
    }

    pub fn index_of(&self, value: &T, from_index: isize) -> isize
    where
        T: JsStrictEqual,
    {
        let state = self.state.borrow();
        let start = normalize_from_index(state.slots.len(), from_index);
        state.slots[start..]
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|item| item.strict_equal(value)))
            .map_or(-1, |index| (start + index) as isize)
    }

    pub fn index_of_from_start(&self, value: &T) -> isize
    where
        T: JsStrictEqual,
    {
        self.index_of(value, 0)
    }

    pub fn join(&self, separator: &str) -> String
    where
        T: crate::string::JsToString,
    {
        self.state
            .borrow()
            .slots
            .iter()
            .map(|slot| {
                slot.as_ref()
                    .map_or_else(String::new, |value| value.to_js_string())
            })
            .collect::<Vec<_>>()
            .join(separator)
    }

    pub fn join_default(&self) -> String
    where
        T: crate::string::JsToString,
    {
        self.join(",")
    }

    pub fn slice(&self, start: f64, end: Option<f64>) -> Self
    where
        T: Clone,
    {
        let state = self.state.borrow();
        let start = crate::coercion::normalize_slice_index(start, state.slots.len());
        let end = end
            .map(|value| crate::coercion::normalize_slice_index(value, state.slots.len()))
            .unwrap_or(state.slots.len());
        if start >= end {
            return Self::new();
        }
        Self::from_slots(state.slots[start..end].to_vec())
    }

    pub fn slice_all(&self) -> Self
    where
        T: Clone,
    {
        self.slice(0.0, None)
    }

    pub fn slice_from(&self, start: f64) -> Self
    where
        T: Clone,
    {
        self.slice(start, None)
    }

    pub fn slice_to(&self, start: f64, end: f64) -> Self
    where
        T: Clone,
    {
        self.slice(start, Some(end))
    }

    pub fn map<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(&T) -> U,
    {
        let length = self.len();
        let output = JsArray::with_length(length);
        for index in 0..length {
            if let Some(value) = self.get(index) {
                output.set(index, mapper(&value));
            }
        }
        output
    }

    pub fn filter<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        let length = self.len();
        let output = Self::new();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(&value) {
                    output.push(value);
                }
            }
        }
        output
    }

    pub fn reduce<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, &T) -> U,
    {
        let length = self.len();
        let mut accumulator = initial;
        for index in 0..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, &value);
            }
        }
        accumulator
    }

    pub fn find<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(&value) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if self.get(index).is_some_and(|value| predicate(&value)) {
                return index as isize;
            }
        }
        -1
    }

    pub fn find_last<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        for index in (0..self.len()).rev() {
            if let Some(value) = self.get(index) {
                if predicate(&value) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_last_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        for index in (0..self.len()).rev() {
            if self.get(index).is_some_and(|value| predicate(&value)) {
                return index as isize;
            }
        }
        -1
    }

    pub fn some<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        let length = self.len();
        (0..length).any(|index| self.get(index).is_some_and(|value| predicate(&value)))
    }

    pub fn every<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(&T) -> bool,
    {
        let length = self.len();
        (0..length).all(|index| self.get(index).is_none_or(|value| predicate(&value)))
    }

    pub fn sort_by_js_string(&self)
    where
        T: Clone + crate::string::JsToString,
    {
        let mut state = self.state.borrow_mut();
        let mut present = state
            .slots
            .iter()
            .filter_map(|slot| slot.as_ref().cloned())
            .collect::<Vec<_>>();
        present.sort_by_key(|item| item.to_js_string());
        let present_len = present.len();
        let length = state.slots.len();
        state.slots = present.into_iter().map(JsSlot::Present).collect();
        state
            .slots
            .resize_with(length.max(present_len), || JsSlot::Hole);
    }

    pub fn to_reversed(&self) -> Self
    where
        T: Clone,
    {
        let output = Self::from_slots(self.state.borrow().slots.clone());
        output.reverse();
        output
    }
}

impl<T> Default for JsArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Iterator for JsArrayIterator<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index >= self.array.len() {
                return None;
            }
            let index = self.index;
            self.index += 1;
            if let Some(value) = self.array.get(index) {
                return Some(value);
            }
        }
    }
}

fn normalize_range(len: usize, start: isize, end: Option<isize>) -> (usize, usize) {
    let start = normalize_index(len, start);
    let end = normalize_index(len, end.unwrap_or(len as isize));
    if end < start {
        (start, start)
    } else {
        (start, end)
    }
}

fn normalize_index(len: usize, index: isize) -> usize {
    let len = len as isize;
    let normalized = if index < 0 { len + index } else { index };
    normalized.clamp(0, len) as usize
}

fn normalize_from_index(len: usize, from_index: isize) -> usize {
    if from_index >= 0 {
        return (from_index as usize).min(len);
    }
    len.saturating_sub(from_index.unsigned_abs())
}
