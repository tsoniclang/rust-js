use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

use super::statics::JsArrayConcatItem;
use crate::coercion::{normalize_slice_index, relative_index, to_integer_or_infinity};
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use tsonic_rust_runtime::{JsError, JsErrorKind, ObjectIdentity, ObjectIdentityCarrier};

#[derive(Debug)]
struct JsArrayState<T> {
    values: Vec<T>,
    numeric_properties: Vec<(String, T)>,
}

impl<T> JsArrayState<T> {
    fn enumerable_own_keys(&self) -> impl Iterator<Item = String> + '_ {
        self.values
            .iter()
            .enumerate()
            .map(|(index, _)| index.to_string())
            .chain(self.numeric_properties.iter().map(|(key, _)| key.clone()))
    }
}

#[derive(Debug)]
pub struct JsArray<T> {
    state: Rc<RefCell<JsArrayState<T>>>,
    identity: ObjectIdentity,
}

pub struct JsArrayIterator<T> {
    array: JsArray<T>,
    index: usize,
}

impl<T> Clone for JsArray<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
            identity: self.identity.clone(),
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

impl<T> JsHash for JsArray<T> {
    fn js_hash(&self) -> u64 {
        hash_identity(self.identity())
    }
}

impl<T> JsStrictEqual for JsArray<T> {
    fn strict_equal(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> JsArray<T> {
    pub fn new() -> Self {
        Self::from_dense(Vec::new())
    }

    pub fn with_length(length: usize) -> Self
    where T: Default,
    {
        Self::from_values(std::iter::repeat_with(T::default).take(length))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_dense(Vec::with_capacity(capacity))
    }

    pub fn from_dense(values: Vec<T>) -> Self {
        Self {
            state: Rc::new(RefCell::new(JsArrayState { values, numeric_properties: Vec::new() })),
            identity: ObjectIdentity::new(),
        }
    }

    pub(super) fn from_values(values: impl IntoIterator<Item = T>) -> Self {
        Self::from_dense(values.into_iter().collect())
    }

    pub(super) fn try_from_values<E>(values: impl IntoIterator<Item = Result<T, E>>) -> Result<Self, E> {
        values.into_iter().collect::<Result<Vec<_>, _>>().map(Self::from_dense)
    }

    pub(super) fn copy_materialized(&self) -> Self
    where T: Clone,
    {
        Self::from_dense(self.state.borrow().values.clone())
    }

    pub(super) fn replace_present_values(&self, values: Vec<T>) {
        let mut state = self.state.borrow_mut();
        for (index, value) in values.into_iter().enumerate() {
            if index < state.values.len() { state.values[index] = value; }
            else { state.values.push(value); }
        }
    }

    pub(super) fn try_sort_present_by<E, F>(&self, mut compare: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(T, T) -> Result<f64, E>,
    {
        let values = self.state.borrow().values.clone();
        let values = try_stable_sort(values, &mut compare)?;
        self.replace_present_values(values);
        Ok(self.clone())
    }

    fn sort_present_by<F>(&self, mut compare: F) -> Self
    where
        T: Clone,
        F: FnMut(T, T) -> f64,
    {
        match self.try_sort_present_by(|left, right| Ok::<_, Infallible>(compare(left, right))) {
            Ok(sorted) => sorted,
            Err(never) => match never {},
        }
    }

    pub fn sort_zero<F>(&self, mut compare: F) -> Self
    where
        T: Clone,
        F: FnMut() -> f64,
    {
        self.sort_present_by(|_, _| compare())
    }

    pub fn sort_value<F>(&self, mut compare: F) -> Self
    where
        T: Clone,
        F: FnMut(T) -> f64,
    {
        self.sort_present_by(|left, _| compare(left))
    }

    pub fn sort<F>(&self, compare: F) -> Self
    where
        T: Clone,
        F: FnMut(T, T) -> f64,
    {
        self.sort_present_by(compare)
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }

    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.state) as usize
    }

    pub fn len(&self) -> usize {
        self.state.borrow().values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_len(&self, len: usize) {
        let mut state = self.state.borrow_mut();
        assert!(len <= state.values.len(), "Dense array growth requires initialized values; use push or fill at construction");
        state.values.truncate(len);
    }

    pub fn has_index(&self, index: usize) -> bool {
        index < self.len()
    }

    pub fn contains_number_property(index: f64, array: &Self) -> bool {
        if let Some(index) = canonical_array_index(index) {
            return array.has_index(index);
        }
        let key = crate::number::to_string(index);
        array
            .state
            .borrow()
            .numeric_properties
            .iter()
            .any(|(candidate, _)| candidate == &key)
    }

    pub fn delete_at(&self, index: usize) -> bool {
        assert!(index >= self.len(), "Deleting a dense array element would create a hole; use splice");
        true
    }

    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.state
            .borrow()
            .values
            .get(index)
            .cloned()
    }

    pub fn get_number(&self, index: f64) -> Option<T>
    where
        T: Clone,
    {
        if let Some(index) = canonical_array_index(index) {
            return self.get(index);
        }
        let key = crate::number::to_string(index);
        self.state
            .borrow()
            .numeric_properties
            .iter()
            .find_map(|(candidate, value)| (candidate == &key).then(|| value.clone()))
    }

    pub fn at(&self, index: f64) -> Option<T>
    where
        T: Clone,
    {
        self.get(relative_index(index, self.len())?)
    }

    pub fn set(&self, index: usize, value: T) {
        let mut state = self.state.borrow_mut();
        assert!(index <= state.values.len(), "Array assignment exceeds initialized dense storage");
        if index == state.values.len() { state.values.push(value); }
        else { state.values[index] = value; }
    }

    pub fn set_number(&self, index: f64, value: T) {
        if let Some(index) = canonical_array_index(index) {
            self.set(index, value);
            return;
        }
        let key = crate::number::to_string(index);
        let mut state = self.state.borrow_mut();
        if let Some((_, current)) = state
            .numeric_properties
            .iter_mut()
            .find(|(candidate, _)| candidate == &key)
        {
            *current = value;
        } else {
            state.numeric_properties.push((key, value));
        }
    }

    pub fn delete_number(&self, index: f64) -> bool {
        if let Some(index) = canonical_array_index(index) {
            return self.delete_at(index);
        }
        let key = crate::number::to_string(index);
        self.state
            .borrow_mut()
            .numeric_properties
            .retain(|(candidate, _)| candidate != &key);
        true
    }

    pub fn push(&self, value: T) -> usize {
        let mut state = self.state.borrow_mut();
        state.values.push(value);
        state.values.len()
    }

    pub fn push_many<const N: usize>(&self, items: [T; N]) -> usize {
        let mut state = self.state.borrow_mut();
        state.values.extend(items);
        state.values.len()
    }

    pub fn push_many_discard<const N: usize>(&self, items: [T; N]) {
        self.state
            .borrow_mut()
            .values
            .extend(items);
    }

    pub fn pop(&self) -> Option<T> {
        self.state.borrow_mut().values.pop()
    }

    pub fn shift(&self) -> Option<T> {
        let mut state = self.state.borrow_mut();
        if state.values.is_empty() {
            return None;
        }
        Some(state.values.remove(0))
    }

    pub fn unshift(&self, value: T) -> usize {
        let mut state = self.state.borrow_mut();
        state.values.insert(0, value);
        state.values.len()
    }

    pub fn unshift_many<const N: usize>(&self, items: [T; N]) -> usize {
        let mut state = self.state.borrow_mut();
        state
            .values
            .splice(0..0, items);
        state.values.len()
    }

    pub fn unshift_many_discard<const N: usize>(&self, items: [T; N]) {
        self.state
            .borrow_mut()
            .values
            .splice(0..0, items);
    }

    pub fn concat<const N: usize>(&self, items: [JsArrayConcatItem<T>; N]) -> Self
    where
        T: Clone,
    {
        let mut values = self.state.borrow().values.clone();
        for item in items {
            match item {
                JsArrayConcatItem::Value(value) => values.push(value),
                JsArrayConcatItem::Array(array) => {
                    values.extend(array.state.borrow().values.iter().cloned());
                }
            }
        }
        Self::from_dense(values)
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
            for slot in &mut state.values[start..end] {
                *slot = value.clone();
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
        let mut state = self.state.borrow_mut();
        if to > from {
            for offset in (0..count).rev() { state.values[to + offset] = state.values[from + offset].clone(); }
        } else {
            for offset in 0..count { state.values[to + offset] = state.values[from + offset].clone(); }
        }
        self.clone()
    }

    pub fn reverse(&self) -> Self {
        self.state.borrow_mut().values.reverse();
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
            .values
            .splice(
                start..start + delete_count,
                items,
            )
            .collect();
        Self::from_dense(removed)
    }

    pub fn keys(&self) -> Vec<usize> {
        (0..self.len()).collect()
    }

    pub fn enumerable_own_keys(&self) -> Vec<String> {
        self.state.borrow().enumerable_own_keys().collect()
    }

    pub fn object_keys(&self) -> JsArray<String> {
        JsArray::from_values(self.state.borrow().enumerable_own_keys())
    }

    pub fn values(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.state
            .borrow()
            .values
            .iter()
            .cloned().map(Some)
            .collect()
    }

    pub fn entries(&self) -> super::JsArrayEntries<T> {
        super::JsArrayEntries::new(self.clone())
    }

    pub fn iter_values(&self) -> JsArrayIterator<T> {
        JsArrayIterator {
            array: self.clone(),
            index: 0,
        }
    }

    pub fn includes<Query: ?Sized>(&self, value: &Query, from_index: f64) -> bool
    where
        T: JsSameValueZero<Query>,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_search_start(state.values.len(), from_index) else {
            return false;
        };
        state.values[start..].iter().any(|item| item.same_value_zero(value))
    }

    pub fn includes_from_start<Query: ?Sized>(&self, value: &Query) -> bool
    where
        T: JsSameValueZero<Query>,
    {
        self.includes(value, 0.0)
    }

    pub fn index_of<Query: ?Sized>(&self, value: &Query, from_index: f64) -> isize
    where
        T: JsStrictEqual<Query>,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_search_start(state.values.len(), from_index) else {
            return -1;
        };
        state.values[start..]
            .iter()
            .position(|item| item.strict_equal(value))
            .map_or(-1, |index| (start + index) as isize)
    }

    pub fn index_of_from_start<Query: ?Sized>(&self, value: &Query) -> isize
    where
        T: JsStrictEqual<Query>,
    {
        self.index_of(value, 0.0)
    }

    pub fn last_index_of<Query: ?Sized>(&self, value: &Query, from_index: f64) -> isize
    where
        T: JsStrictEqual<Query>,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_last_search_start(state.values.len(), from_index) else {
            return -1;
        };
        state.values[..=start]
            .iter()
            .rposition(|item| item.strict_equal(value))
            .map_or(-1, |index| index as isize)
    }

    pub fn last_index_of_from_end<Query: ?Sized>(&self, value: &Query) -> isize
    where
        T: JsStrictEqual<Query>,
    {
        self.last_index_of(value, f64::INFINITY)
    }

    pub fn join(&self, separator: &str) -> String
    where
        T: crate::string::JsToString,
    {
        self.state
            .borrow()
            .values
            .iter()
            .map(|value| value.to_js_string())
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
        let start = crate::coercion::normalize_slice_index(start, state.values.len());
        let end = end
            .map(|value| crate::coercion::normalize_slice_index(value, state.values.len()))
            .unwrap_or(state.values.len());
        if start >= end {
            return Self::new();
        }
        Self::from_dense(state.values[start..end].to_vec())
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
        let output = JsArray::with_capacity(length);
        for index in 0..length {
            let value = self.get(index).expect("Array.map cannot create holes after its source is shortened");
            output.push(mapper(value, index as f64, self.clone()));
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
        self.state.borrow_mut().values.sort_by_cached_key(|item| item.to_js_string());
        self.clone()
    }

    pub fn to_reversed(&self) -> Self
    where
        T: Clone,
    {
        let output = Self::from_dense(self.state.borrow().values.clone());
        output.reverse();
        output
    }
}

fn try_stable_sort<T, E, F>(mut values: Vec<T>, compare: &mut F) -> Result<Vec<T>, E>
where
    T: Clone,
    F: FnMut(T, T) -> Result<f64, E>,
{
    if values.len() <= 1 {
        return Ok(values);
    }
    let length = values.len();
    let mut order: Vec<usize> = (0..length).collect();
    let mut scratch = vec![0; length];
    let mut width = 1usize;
    while width < length {
        let mut start = 0;
        while start < length {
            let middle = start.saturating_add(width).min(length);
            let end = middle.saturating_add(width).min(length);
            let mut left = start;
            let mut right = middle;
            let mut output = start;
            while left < middle && right < end {
                let comparison = compare(values[order[left]].clone(), values[order[right]].clone())?;
                if comparison.is_nan() || comparison <= 0.0 {
                    scratch[output] = order[left];
                    left += 1;
                } else {
                    scratch[output] = order[right];
                    right += 1;
                }
                output += 1;
            }
            let remaining = if left < middle { &order[left..middle] } else { &order[right..end] };
            scratch[output..end].copy_from_slice(remaining);
            start = end;
        }
        std::mem::swap(&mut order, &mut scratch);
        width = width.saturating_mul(2);
    }
    for (destination, source) in order.into_iter().enumerate() {
        scratch[source] = destination;
    }
    for index in 0..length {
        while scratch[index] != index {
            let destination = scratch[index];
            values.swap(index, destination);
            scratch.swap(index, destination);
        }
    }
    Ok(values)
}

pub(super) fn canonical_array_index(value: f64) -> Option<usize> {
    const MAX_ARRAY_INDEX: f64 = 4_294_967_294.0;
    (value.is_finite() && (0.0..=MAX_ARRAY_INDEX).contains(&value) && value.trunc() == value)
        .then_some(value as usize)
}

impl<T> ObjectIdentityCarrier for JsArray<T> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
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
