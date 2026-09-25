mod callbacks;

use std::cell::{OnceCell, Ref, RefCell};
use std::convert::Infallible;
use std::rc::Rc;

use super::statics::JsArrayConcatItem;
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::native_integer::{normalize_slice_index, relative_index};
use crate::numeric::IndexInput;
use crate::string::JsToString;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

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
struct JsArrayOwner<T> {
    values: RefCell<JsArrayState<T>>,
    identity: OnceCell<ObjectIdentity>,
}

impl<T> std::ops::Deref for JsArrayOwner<T> {
    type Target = RefCell<JsArrayState<T>>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

#[derive(Debug)]
pub struct JsArray<T> {
    state: Rc<JsArrayOwner<T>>,
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
    where
        T: Default,
    {
        Self::from_values(std::iter::repeat_with(T::default).take(length))
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_dense(Vec::with_capacity(capacity))
    }

    pub fn from_dense(values: Vec<T>) -> Self {
        Self {
            state: Rc::new(JsArrayOwner {
                values: RefCell::new(JsArrayState {
                    values,
                    numeric_properties: Vec::new(),
                }),
                identity: OnceCell::new(),
            }),
        }
    }

    pub(super) fn from_values(values: impl IntoIterator<Item = T>) -> Self {
        Self::from_dense(values.into_iter().collect())
    }

    pub(super) fn try_from_values<E>(
        values: impl IntoIterator<Item = Result<T, E>>,
    ) -> Result<Self, E> {
        values
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map(Self::from_dense)
    }

    pub(super) fn copy_materialized(&self) -> Self
    where
        T: Clone,
    {
        Self::from_dense(self.state.borrow().values.clone())
    }

    pub(super) fn replace_present_values(&self, values: Vec<T>) {
        let mut state = self.state.borrow_mut();
        for (index, value) in values.into_iter().enumerate() {
            if index < state.values.len() {
                state.values[index] = value;
            } else {
                state.values.push(value);
            }
        }
    }

    pub(super) fn try_sort_present_by<E, F>(&self, mut compare: F) -> Result<Self, E>
    where
        T: Clone,
        F: FnMut(&T, &T) -> Result<f64, E>,
    {
        let values = self.state.borrow().values.clone();
        let values = try_stable_sort(values, &mut compare)?;
        self.replace_present_values(values);
        Ok(self.clone())
    }

    fn sort_present_by<F>(&self, mut compare: F) -> Self
    where
        T: Clone,
        F: FnMut(&T, &T) -> f64,
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
        self.sort_present_by(|left, _| compare(left.clone()))
    }

    pub fn sort<F>(&self, mut compare: F) -> Self
    where
        T: Clone,
        F: FnMut(T, T) -> f64,
    {
        self.sort_present_by(|left, right| compare(left.clone(), right.clone()))
    }

    pub fn sort_borrowed<F>(&self, mut compare: F) -> Self
    where
        T: Clone + AsRef<str>,
        F: FnMut(&str, &str) -> f64,
    {
        self.sort_present_by(|left, right| compare(left.as_ref(), right.as_ref()))
    }

    pub fn sort_value_borrowed<F>(&self, mut compare: F) -> Self
    where
        T: Clone + AsRef<str>,
        F: FnMut(&str) -> f64,
    {
        self.sort_present_by(|left, _| compare(left.as_ref()))
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

    pub fn set_len(&self, len: impl IndexInput) {
        let len = len
            .checked_integer()
            .expect("Array length must be an exact native index");
        let mut state = self.state.borrow_mut();
        assert!(
            len <= state.values.len(),
            "Dense array growth requires initialized values; use push or fill at construction"
        );
        state.values.truncate(len);
    }

    pub fn has_index(&self, index: usize) -> bool {
        index < self.len()
    }

    pub fn contains_number_property(index: impl IndexInput + JsToString, array: &Self) -> bool {
        if let Some(index) = canonical_array_index(index) {
            return array.has_index(index);
        }
        let key = index.to_js_string();
        array
            .state
            .borrow()
            .numeric_properties
            .iter()
            .any(|(candidate, _)| candidate == &key)
    }

    pub fn delete_at(&self, index: usize) -> bool {
        assert!(
            index >= self.len(),
            "Deleting a dense array element would create a hole; use splice"
        );
        true
    }

    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.state.borrow().values.get(index).cloned()
    }

    pub(crate) fn with_element<Result>(
        &self,
        index: usize,
        read: impl FnOnce(Option<&T>) -> Result,
    ) -> Result {
        read(self.state.borrow().values.get(index))
    }

    pub fn get_number(&self, index: impl IndexInput + JsToString) -> Option<T>
    where
        T: Clone,
    {
        if let Some(index) = canonical_array_index(index) {
            return self.get(index);
        }
        let key = index.to_js_string();
        self.state
            .borrow()
            .numeric_properties
            .iter()
            .find_map(|(candidate, value)| (candidate == &key).then(|| value.clone()))
    }

    pub fn borrow_number_element(&self, index: impl IndexInput + JsToString) -> Option<Ref<'_, T>> {
        if let Some(index) = canonical_array_index(index) {
            return Ref::filter_map(self.state.borrow(), |state| state.values.get(index)).ok();
        }
        let key = index.to_js_string();
        Ref::filter_map(self.state.borrow(), |state| {
            state
                .numeric_properties
                .iter()
                .find_map(|(candidate, value)| (candidate == &key).then_some(value))
        })
        .ok()
    }

    pub fn at(&self, index: impl IndexInput) -> Option<T>
    where
        T: Clone,
    {
        self.get(relative_index(index, self.len())?)
    }

    pub fn set(&self, index: usize, value: T) {
        let mut state = self.state.borrow_mut();
        assert!(
            index <= state.values.len(),
            "Array assignment exceeds initialized dense storage"
        );
        if index == state.values.len() {
            state.values.push(value);
        } else {
            state.values[index] = value;
        }
    }

    pub fn set_number(&self, index: impl IndexInput + JsToString, value: T) {
        if let Some(index) = canonical_array_index(index) {
            self.set(index, value);
            return;
        }
        let key = index.to_js_string();
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

    pub fn delete_number(&self, index: impl IndexInput + JsToString) -> bool {
        if let Some(index) = canonical_array_index(index) {
            return self.delete_at(index);
        }
        let key = index.to_js_string();
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

    pub fn push_many(&self, items: impl IntoIterator<Item = T>) -> usize {
        let mut state = self.state.borrow_mut();
        state.values.extend(items);
        state.values.len()
    }

    pub fn push_many_discard(&self, items: impl IntoIterator<Item = T>) {
        self.state.borrow_mut().values.extend(items);
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

    pub fn unshift_many(&self, items: impl IntoIterator<Item = T>) -> usize {
        let mut state = self.state.borrow_mut();
        state.values.splice(0..0, items);
        state.values.len()
    }

    pub fn unshift_many_discard(&self, items: impl IntoIterator<Item = T>) {
        self.state.borrow_mut().values.splice(0..0, items);
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
        self.fill(value, 0_usize, None::<usize>)
    }

    pub fn fill_from(&self, value: T, start: impl IndexInput) -> Self
    where
        T: Clone,
    {
        self.fill(value, start, None::<usize>)
    }

    pub fn fill_to(&self, value: T, start: impl IndexInput, end: impl IndexInput) -> Self
    where
        T: Clone,
    {
        self.fill(value, start, Some(end))
    }

    fn fill(&self, value: T, start: impl IndexInput, end: Option<impl IndexInput>) -> Self
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

    pub fn copy_within_from(&self, target: impl IndexInput, start: impl IndexInput) -> Self
    where
        T: Clone,
    {
        self.copy_within(target, start, None::<usize>)
    }

    pub fn copy_within_to(
        &self,
        target: impl IndexInput,
        start: impl IndexInput,
        end: impl IndexInput,
    ) -> Self
    where
        T: Clone,
    {
        self.copy_within(target, start, Some(end))
    }

    fn copy_within(
        &self,
        target: impl IndexInput,
        start: impl IndexInput,
        end: Option<impl IndexInput>,
    ) -> Self
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
            for offset in (0..count).rev() {
                state.values[to + offset] = state.values[from + offset].clone();
            }
        } else {
            for offset in 0..count {
                state.values[to + offset] = state.values[from + offset].clone();
            }
        }
        self.clone()
    }

    pub fn reverse(&self) -> Self {
        self.state.borrow_mut().values.reverse();
        self.clone()
    }

    pub fn splice_from(&self, start: impl IndexInput) -> Self {
        self.splice(start, usize::MAX, std::iter::empty())
    }

    pub fn splice_many(
        &self,
        start: impl IndexInput,
        delete_count: impl IndexInput,
        items: impl IntoIterator<Item = T>,
    ) -> Self {
        self.splice(start, delete_count, items)
    }

    fn splice(
        &self,
        start: impl IndexInput,
        delete_count: impl IndexInput,
        items: impl IntoIterator<Item = T>,
    ) -> Self {
        let len = self.len();
        let start = normalize_slice_index(start, len);
        let delete_count = delete_count.positive_index(len.saturating_sub(start));
        let removed = self
            .state
            .borrow_mut()
            .values
            .splice(start..start + delete_count, items)
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

    pub fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.state.borrow().values.clone()
    }

    pub fn with_values<Result>(&self, read: impl FnOnce(&[T]) -> Result) -> Result {
        read(&self.state.borrow().values)
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

    pub fn includes<Query: ?Sized>(&self, value: &Query, from_index: impl IndexInput) -> bool
    where
        T: JsSameValueZero<Query>,
    {
        let state = self.state.borrow();
        let Some(start) = normalize_search_start(state.values.len(), from_index) else {
            return false;
        };
        state.values[start..]
            .iter()
            .any(|item| item.same_value_zero(value))
    }

    pub fn includes_from_start<Query: ?Sized>(&self, value: &Query) -> bool
    where
        T: JsSameValueZero<Query>,
    {
        self.includes(value, 0.0)
    }

    pub fn index_of<Query: ?Sized>(&self, value: &Query, from_index: impl IndexInput) -> isize
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
        self.index_of(value, 0_usize)
    }

    pub fn last_index_of<Query: ?Sized>(&self, value: &Query, from_index: impl IndexInput) -> isize
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
        self.last_index_of(value, usize::MAX)
    }

    pub fn join(&self, separator: &str) -> String
    where
        T: crate::string::JsToString,
    {
        let state = self.state.borrow();
        T::join_js_strings(&state.values, separator)
    }

    pub fn join_default(&self) -> String
    where
        T: crate::string::JsToString,
    {
        self.join(",")
    }

    pub fn slice(&self, start: impl IndexInput, end: Option<impl IndexInput>) -> Self
    where
        T: Clone,
    {
        let state = self.state.borrow();
        let start = crate::native_integer::normalize_slice_index(start, state.values.len());
        let end = end
            .map(|value| crate::native_integer::normalize_slice_index(value, state.values.len()))
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
        self.slice(0_usize, None::<usize>)
    }

    pub fn slice_from(&self, start: impl IndexInput) -> Self
    where
        T: Clone,
    {
        self.slice(start, None::<usize>)
    }

    pub fn slice_to(&self, start: impl IndexInput, end: impl IndexInput) -> Self
    where
        T: Clone,
    {
        self.slice(start, Some(end))
    }

    pub fn sort_by_js_string(&self) -> Self
    where
        T: Clone + crate::string::JsToString,
    {
        self.state
            .borrow_mut()
            .values
            .sort_by_cached_key(|item| item.to_js_string());
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
    F: FnMut(&T, &T) -> Result<f64, E>,
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
                let comparison = compare(&values[order[left]], &values[order[right]])?;
                if comparison.is_nan() || comparison <= 0.0 {
                    scratch[output] = order[left];
                    left += 1;
                } else {
                    scratch[output] = order[right];
                    right += 1;
                }
                output += 1;
            }
            let remaining = if left < middle {
                &order[left..middle]
            } else {
                &order[right..end]
            };
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

pub(super) fn canonical_array_index(value: impl IndexInput) -> Option<usize> {
    value.property_index()
}

impl<T> ObjectIdentityCarrier for JsArray<T> {
    fn object_identity(&self) -> &ObjectIdentity {
        self.state.identity.get_or_init(ObjectIdentity::new)
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

fn normalize_search_start(len: usize, from_index: impl IndexInput) -> Option<usize> {
    let position = normalize_slice_index(from_index, len);
    (position < len).then_some(position)
}

fn normalize_last_search_start(len: usize, from_index: impl IndexInput) -> Option<usize> {
    from_index.last_index(len)
}
