use std::cell::RefCell;
use std::rc::Rc;

use super::slot::JsSlot;
use super::statics::JsArrayConcatItem;
use crate::coercion::{normalize_slice_index, relative_index, to_integer_or_infinity};
use crate::equality::{JsSameValueZero, JsStrictEqual};
use tsonic_rust_runtime::{JsError, JsErrorKind};

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

    pub fn push_many<const N: usize>(&self, items: [T; N]) -> usize {
        let mut state = self.state.borrow_mut();
        state.slots.extend(items.into_iter().map(JsSlot::Present));
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

    pub fn unshift_many<const N: usize>(&self, items: [T; N]) -> usize {
        let mut state = self.state.borrow_mut();
        state
            .slots
            .splice(0..0, items.into_iter().map(JsSlot::Present));
        state.slots.len()
    }

    pub fn concat<const N: usize>(&self, items: [JsArrayConcatItem<T>; N]) -> Self
    where
        T: Clone,
    {
        let mut slots = self.state.borrow().slots.clone();
        for item in items {
            match item {
                JsArrayConcatItem::Value(value) => slots.push(JsSlot::Present(value)),
                JsArrayConcatItem::Array(array) => {
                    slots.extend(array.state.borrow().slots.iter().cloned());
                }
            }
        }
        Self::from_slots(slots)
    }

    pub fn fill_all(&self, value: T) -> Self
    where
        T: Clone,
    {
        self.fill(value, 0.0, None)
    }

    pub fn fill_from(&self, value: T, start: f64) -> Self
    where
        T: Clone,
    {
        self.fill(value, start, None)
    }

    pub fn fill_to(&self, value: T, start: f64, end: f64) -> Self
    where
        T: Clone,
    {
        self.fill(value, start, Some(end))
    }

    fn fill(&self, value: T, start: f64, end: Option<f64>) -> Self
    where
        T: Clone,
    {
        let length = self.len();
        let start = normalize_slice_index(start, length);
        let end = end
            .map(|value| normalize_slice_index(value, length))
            .unwrap_or(length);
        let mut state = self.state.borrow_mut();
        if start < end {
            for slot in &mut state.slots[start..end] {
                *slot = JsSlot::Present(value.clone());
            }
        }
        self.clone()
    }

    pub fn copy_within_from(&self, target: f64, start: f64) -> Self
    where
        T: Clone,
    {
        self.copy_within(target, start, None)
    }

    pub fn copy_within_to(&self, target: f64, start: f64, end: f64) -> Self
    where
        T: Clone,
    {
        self.copy_within(target, start, Some(end))
    }

    fn copy_within(&self, target: f64, start: f64, end: Option<f64>) -> Self
    where
        T: Clone,
    {
        let len = self.len();
        let to = normalize_slice_index(target, len);
        let from = normalize_slice_index(start, len);
        let end = end
            .map(|value| normalize_slice_index(value, len))
            .unwrap_or(len);
        let count = end.saturating_sub(from).min(len.saturating_sub(to));
        let copied = self.state.borrow().slots[from..from + count].to_vec();
        let mut state = self.state.borrow_mut();
        for (offset, slot) in copied.into_iter().enumerate() {
            state.slots[to + offset] = slot;
        }
        self.clone()
    }

    pub fn reverse(&self) -> Self {
        self.state.borrow_mut().slots.reverse();
        self.clone()
    }

    pub fn splice_from(&self, start: f64) -> Self {
        self.splice(start, f64::INFINITY, std::iter::empty())
    }

    pub fn splice_many<const N: usize>(
        &self,
        start: f64,
        delete_count: f64,
        items: [T; N],
    ) -> Self {
        self.splice(start, delete_count, items)
    }

    fn splice(&self, start: f64, delete_count: f64, items: impl IntoIterator<Item = T>) -> Self {
        let len = self.len();
        let start = normalize_slice_index(start, len);
        let delete_count = to_integer_or_infinity(delete_count);
        let delete_count = if delete_count <= 0.0 {
            0
        } else if delete_count == f64::INFINITY {
            len.saturating_sub(start)
        } else {
            (delete_count as usize).min(len.saturating_sub(start))
        };
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

    pub fn includes(&self, value: &T, from_index: f64) -> bool
    where
        T: JsSameValueZero,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_search_start(state.slots.len(), from_index) else {
            return false;
        };
        state.slots[start..].iter().any(|slot| {
            slot.as_ref()
                .is_some_and(|item| item.same_value_zero(value))
        })
    }

    pub fn includes_from_start(&self, value: &T) -> bool
    where
        T: JsSameValueZero,
    {
        self.includes(value, 0.0)
    }

    pub fn index_of(&self, value: &T, from_index: f64) -> isize
    where
        T: JsStrictEqual,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_search_start(state.slots.len(), from_index) else {
            return -1;
        };
        state.slots[start..]
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|item| item.strict_equal(value)))
            .map_or(-1, |index| (start + index) as isize)
    }

    pub fn index_of_from_start(&self, value: &T) -> isize
    where
        T: JsStrictEqual,
    {
        self.index_of(value, 0.0)
    }

    pub fn last_index_of(&self, value: &T, from_index: f64) -> isize
    where
        T: JsStrictEqual,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_last_search_start(state.slots.len(), from_index) else {
            return -1;
        };
        state.slots[..=start]
            .iter()
            .rposition(|slot| slot.as_ref().is_some_and(|item| item.strict_equal(value)))
            .map_or(-1, |index| index as isize)
    }

    pub fn last_index_of_from_end(&self, value: &T) -> isize
    where
        T: JsStrictEqual,
    {
        self.last_index_of(value, f64::INFINITY)
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

    fn map_with<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> U,
    {
        let length = self.len();
        let output = JsArray::with_length(length);
        for index in 0..length {
            if let Some(value) = self.get(index) {
                output.set(index, mapper(value, index as f64, self.clone()));
            }
        }
        output
    }

    pub fn map_zero<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut() -> U,
    {
        self.map_with(|_, _, _| mapper())
    }

    pub fn map<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T) -> U,
    {
        self.map_with(|value, _, _| mapper(value))
    }

    pub fn map_with_index<U, F>(&self, mut mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, f64) -> U,
    {
        self.map_with(|value, index, _| mapper(value, index))
    }

    pub fn map_with_array<U, F>(&self, mapper: F) -> JsArray<U>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> U,
    {
        self.map_with(mapper)
    }

    fn filter_with<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        let length = self.len();
        let output = Self::new();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index as f64, self.clone()) {
                    output.push(value);
                }
            }
        }
        output
    }

    pub fn filter_zero<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.filter_with(|_, _, _| predicate())
    }

    pub fn filter<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.filter_with(|value, _, _| predicate(value))
    }

    pub fn filter_with_index<F>(&self, mut predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.filter_with(|value, index, _| predicate(value, index))
    }

    pub fn filter_with_array<F>(&self, predicate: F) -> Self
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.filter_with(predicate)
    }

    fn reduce_with<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> U,
    {
        let length = self.len();
        let mut accumulator = initial;
        for index in 0..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index as f64, self.clone());
            }
        }
        accumulator
    }

    pub fn reduce_zero<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut() -> U,
    {
        self.reduce_with(initial, |_, _, _, _| reducer())
    }

    pub fn reduce_accumulator<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U) -> U,
    {
        self.reduce_with(initial, |accumulator, _, _, _| reducer(accumulator))
    }

    pub fn reduce<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T) -> U,
    {
        self.reduce_with(initial, |accumulator, value, _, _| {
            reducer(accumulator, value)
        })
    }

    pub fn reduce_with_index<U, F>(&self, initial: U, mut reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, f64) -> U,
    {
        self.reduce_with(initial, |accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn reduce_with_array<U, F>(&self, initial: U, reducer: F) -> U
    where
        T: Clone,
        F: FnMut(U, T, f64, Self) -> U,
    {
        self.reduce_with(initial, reducer)
    }

    fn reduce_from_first_with<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, f64, Self) -> T,
    {
        let length = self.len();
        let Some((first_index, mut accumulator)) =
            (0..length).find_map(|index| self.get(index).map(|value| (index, value)))
        else {
            return Err(JsError::new(
                JsErrorKind::TypeError,
                "Reduce of empty array with no initial value",
            ));
        };
        for index in first_index + 1..length {
            if let Some(value) = self.get(index) {
                accumulator = reducer(accumulator, value, index as f64, self.clone());
            }
        }
        Ok(accumulator)
    }

    pub fn reduce_from_first_zero<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut() -> T,
    {
        self.reduce_from_first_with(|_, _, _, _| reducer())
    }

    pub fn reduce_from_first_accumulator<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T) -> T,
    {
        self.reduce_from_first_with(|accumulator, _, _, _| reducer(accumulator))
    }

    pub fn reduce_from_first<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T) -> T,
    {
        self.reduce_from_first_with(|accumulator, value, _, _| reducer(accumulator, value))
    }

    pub fn reduce_from_first_with_index<F>(&self, mut reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, f64) -> T,
    {
        self.reduce_from_first_with(|accumulator, value, index, _| {
            reducer(accumulator, value, index)
        })
    }

    pub fn reduce_from_first_with_array<F>(&self, reducer: F) -> Result<T, JsError>
    where
        T: Clone,
        F: FnMut(T, T, f64, Self) -> T,
    {
        self.reduce_from_first_with(reducer)
    }

    pub fn for_each_zero<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(),
    {
        self.for_each_with(|_, _, _| callback());
    }

    pub fn for_each_value<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T),
    {
        self.for_each_with(|value, _, _| callback(value));
    }

    pub fn for_each_value_index<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, f64),
    {
        self.for_each_with(|value, index, _| callback(value, index));
    }

    pub fn for_each<F>(&self, callback: F)
    where
        T: Clone,
        F: FnMut(T, f64, Self),
    {
        self.for_each_with(callback);
    }

    fn for_each_with<F>(&self, mut callback: F)
    where
        T: Clone,
        F: FnMut(T, f64, Self),
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                callback(value, index as f64, self.clone());
            }
        }
    }

    fn find_with<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index as f64, self.clone()) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_zero<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_with(|_, _, _| predicate())
    }

    pub fn find<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_with(|value, _, _| predicate(value))
    }

    pub fn find_with_index<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.find_with(|value, index, _| predicate(value, index))
    }

    pub fn find_with_array<F>(&self, predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.find_with(predicate)
    }

    fn find_index_with<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        let length = self.len();
        for index in 0..length {
            if let Some(value) = self.get(index) {
                if predicate(value, index as f64, self.clone()) {
                    return index as isize;
                }
            }
        }
        -1
    }

    pub fn find_index_zero<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_index_with(|_, _, _| predicate())
    }

    pub fn find_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_index_with(|value, _, _| predicate(value))
    }

    pub fn find_index_with_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.find_index_with(|value, index, _| predicate(value, index))
    }

    pub fn find_index_with_array<F>(&self, predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.find_index_with(predicate)
    }

    fn find_last_with<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        for index in (0..self.len()).rev() {
            if let Some(value) = self.get(index) {
                if predicate(value.clone(), index as f64, self.clone()) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn find_last_zero<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_last_with(|_, _, _| predicate())
    }

    pub fn find_last<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_last_with(|value, _, _| predicate(value))
    }

    pub fn find_last_with_index<F>(&self, mut predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.find_last_with(|value, index, _| predicate(value, index))
    }

    pub fn find_last_with_array<F>(&self, predicate: F) -> Option<T>
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.find_last_with(predicate)
    }

    fn find_last_index_with<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        for index in (0..self.len()).rev() {
            if let Some(value) = self.get(index) {
                if predicate(value, index as f64, self.clone()) {
                    return index as isize;
                }
            }
        }
        -1
    }

    pub fn find_last_index_zero<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.find_last_index_with(|_, _, _| predicate())
    }

    pub fn find_last_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.find_last_index_with(|value, _, _| predicate(value))
    }

    pub fn find_last_index_with_index<F>(&self, mut predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.find_last_index_with(|value, index, _| predicate(value, index))
    }

    pub fn find_last_index_with_array<F>(&self, predicate: F) -> isize
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.find_last_index_with(predicate)
    }

    fn some_with<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        let length = self.len();
        (0..length).any(|index| {
            self.get(index)
                .is_some_and(|value| predicate(value, index as f64, self.clone()))
        })
    }

    pub fn some_zero<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.some_with(|_, _, _| predicate())
    }

    pub fn some<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.some_with(|value, _, _| predicate(value))
    }

    pub fn some_with_index<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.some_with(|value, index, _| predicate(value, index))
    }

    pub fn some_with_array<F>(&self, predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.some_with(predicate)
    }

    fn every_with<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        let length = self.len();
        (0..length).all(|index| {
            self.get(index)
                .is_none_or(|value| predicate(value, index as f64, self.clone()))
        })
    }

    pub fn every_zero<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut() -> bool,
    {
        self.every_with(|_, _, _| predicate())
    }

    pub fn every<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T) -> bool,
    {
        self.every_with(|value, _, _| predicate(value))
    }

    pub fn every_with_index<F>(&self, mut predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64) -> bool,
    {
        self.every_with(|value, index, _| predicate(value, index))
    }

    pub fn every_with_array<F>(&self, predicate: F) -> bool
    where
        T: Clone,
        F: FnMut(T, f64, Self) -> bool,
    {
        self.every_with(predicate)
    }

    pub fn sort_by_js_string(&self) -> Self
    where
        T: Clone + crate::string::JsToString,
    {
        let mut state = self.state.borrow_mut();
        let mut present = state
            .slots
            .iter()
            .filter_map(|slot| slot.as_ref().cloned())
            .collect::<Vec<_>>();
        present.sort_by_key(|item| item.to_js_string().encode_utf16().collect::<Vec<_>>());
        let present_len = present.len();
        let length = state.slots.len();
        state.slots = present.into_iter().map(JsSlot::Present).collect();
        state
            .slots
            .resize_with(length.max(present_len), || JsSlot::Hole);
        self.clone()
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

fn normalize_search_start(len: usize, from_index: f64) -> Option<usize> {
    let from_index = to_integer_or_infinity(from_index);
    if from_index == f64::INFINITY || from_index >= len as f64 {
        return None;
    }
    if from_index == f64::NEG_INFINITY {
        return Some(0);
    }
    if from_index >= 0.0 {
        return Some(from_index as usize);
    }
    Some((len as f64 + from_index).max(0.0) as usize)
}

fn normalize_last_search_start(len: usize, from_index: f64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let from_index = to_integer_or_infinity(from_index);
    if from_index == f64::NEG_INFINITY {
        return None;
    }
    if from_index >= 0.0 {
        return Some((from_index as usize).min(len - 1));
    }
    let index = len as f64 + from_index;
    (index >= 0.0).then_some(index as usize)
}
