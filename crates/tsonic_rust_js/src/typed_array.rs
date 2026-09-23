//! JavaScript typed-array carriers with exact numeric conversion, copy `slice`,
//! and shared-backing-store `subarray` semantics.

pub use elements::ConvertElement;
use std::cmp::Ordering;
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

use crate::array::JsArray;
use crate::array_buffer::{normalize_index, to_index, ArrayBuffer};
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};

mod elements;
pub use elements::{ClampedU8, TypedArrayKind, TypedElement};

#[derive(Debug)]
struct TypedArrayView {
    buffer: ArrayBuffer,
    byte_offset: usize,
    length: usize,
    identity: ObjectIdentity,
}

#[derive(Debug, Clone)]
pub struct TypedArray<T: TypedElement> {
    view: Rc<TypedArrayView>,
    element: std::marker::PhantomData<T>,
}

pub type Int8Array = TypedArray<i8>;
pub type Uint8Array = TypedArray<u8>;
pub type Uint8ClampedArray = TypedArray<ClampedU8>;
pub type Int16Array = TypedArray<i16>;
pub type Uint16Array = TypedArray<u16>;
pub type Int32Array = TypedArray<i32>;
pub type Uint32Array = TypedArray<u32>;
pub type Float32Array = TypedArray<f32>;
pub type Float64Array = TypedArray<f64>;

impl TypedArray<u8> {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let length = bytes.len();
        Self::from_view(ArrayBuffer::from_bytes(bytes), 0, length)
    }
}

impl<T: TypedElement> TypedArray<T> {
    pub const BYTES_PER_ELEMENT: usize = T::BYTES_PER_ELEMENT;

    pub fn new(length: f64) -> JsResult<Self> {
        Self::with_length(to_index(length)?)
    }

    fn with_length(length: usize) -> JsResult<Self> {
        let byte_length = length
            .checked_mul(T::BYTES_PER_ELEMENT)
            .ok_or_else(|| range_error("typed array length is outside the supported range"))?;
        Ok(Self::from_view(
            ArrayBuffer::from_bytes(vec![0; byte_length]),
            0,
            length,
        ))
    }

    pub fn from_array(values: &JsArray<f64>) -> JsResult<Self> {
        Self::from_numbers(values.iter_values())
    }

    pub fn from_typed_array<U: TypedElement + ConvertElement<T>>(
        values: &TypedArray<U>,
    ) -> JsResult<Self> {
        let result = Self::with_length(values.len())?;
        result.set_from_typed_array(values, 0.0)?;
        Ok(result)
    }

    pub fn from_fixed_array<const LENGTH: usize>(values: &[f64; LENGTH]) -> JsResult<Self> {
        Self::from_numbers(values.iter().copied())
    }

    pub fn from_vec(values: Vec<f64>) -> JsResult<Self> {
        Self::from_numbers(values)
    }

    pub fn from_numbers(values: impl IntoIterator<Item = f64>) -> JsResult<Self> {
        let values: Vec<f64> = values.into_iter().collect();
        let result = Self::with_length(values.len())?;
        for (index, value) in values.into_iter().enumerate() {
            result.set_usize(index, T::from_number(value));
        }
        Ok(result)
    }

    pub fn from_buffer(
        buffer: ArrayBuffer,
        byte_offset: f64,
        length: Option<f64>,
    ) -> JsResult<Self> {
        let byte_offset = to_index(byte_offset)?;
        let buffer_length = buffer.byte_length();
        if byte_offset > buffer_length || byte_offset % T::BYTES_PER_ELEMENT != 0 {
            return Err(range_error("typed array byte offset is invalid"));
        }
        let available = buffer_length - byte_offset;
        let length = match length {
            Some(value) => to_index(value)?,
            None if available.is_multiple_of(T::BYTES_PER_ELEMENT) => {
                available / T::BYTES_PER_ELEMENT
            }
            None => return Err(range_error("typed array buffer length is invalid")),
        };
        let byte_length = length
            .checked_mul(T::BYTES_PER_ELEMENT)
            .ok_or_else(|| range_error("typed array length is outside the supported range"))?;
        if byte_length > available {
            return Err(range_error("typed array view exceeds its ArrayBuffer"));
        }
        Ok(Self::from_view(buffer, byte_offset, length))
    }

    pub fn from_buffer_only(buffer: ArrayBuffer) -> JsResult<Self> {
        Self::from_buffer(buffer, 0.0, None)
    }

    pub fn from_buffer_offset(buffer: ArrayBuffer, byte_offset: f64) -> JsResult<Self> {
        Self::from_buffer(buffer, byte_offset, None)
    }

    pub fn from_buffer_length(
        buffer: ArrayBuffer,
        byte_offset: f64,
        length: f64,
    ) -> JsResult<Self> {
        Self::from_buffer(buffer, byte_offset, Some(length))
    }

    pub fn buffer(&self) -> ArrayBuffer {
        self.view.buffer.clone()
    }

    pub fn with_bytes<Result>(&self, operation: impl FnOnce(&[u8]) -> Result) -> Result {
        let bytes = self.view.buffer.as_bytes();
        let start = self.view.byte_offset;
        let end = start + self.view.length * T::BYTES_PER_ELEMENT;
        operation(&bytes[start..end])
    }

    pub fn with_mut_bytes<Result>(&self, operation: impl FnOnce(&mut [u8]) -> Result) -> Result {
        let mut bytes = self.view.buffer.as_mut_bytes();
        let start = self.view.byte_offset;
        let end = start + self.view.length * T::BYTES_PER_ELEMENT;
        operation(&mut bytes[start..end])
    }

    pub fn bytes_per_element(&self) -> usize {
        Self::BYTES_PER_ELEMENT
    }

    pub fn byte_length(&self) -> usize {
        self.view.length * T::BYTES_PER_ELEMENT
    }

    pub fn byte_offset(&self) -> usize {
        self.view.byte_offset
    }

    pub fn length(&self) -> usize {
        self.view.length
    }

    pub fn len(&self) -> usize {
        self.view.length
    }

    pub fn is_empty(&self) -> bool {
        self.view.length == 0
    }

    pub fn at(&self, index: f64) -> Option<T::Value> {
        self.get_usize(crate::native_integer::relative_index(
            index,
            self.view.length,
        )?)
        .map(TypedElement::into_value)
    }

    pub fn get_number(&self, index: f64) -> Option<T::Value> {
        if !index.is_finite() || index.fract() != 0.0 {
            return None;
        }
        let index = crate::native_integer::absolute_index(index, self.view.length)?;
        self.get_usize(index).map(TypedElement::into_value)
    }

    pub fn set_number(&self, index: f64, value: f64) {
        if index.is_finite() && index >= 0.0 && index.fract() == 0.0 {
            self.set_usize(index as usize, T::from_number(value));
        }
    }

    pub fn fill(&self, value: f64, start: f64, end: Option<f64>) -> Self {
        let (start, end) = normalized_range(self.view.length, start, end);
        let value = T::from_number(value);
        let mut storage = self.view.buffer.as_mut_bytes();
        let bytes = &mut storage[self.view.byte_offset + start * T::BYTES_PER_ELEMENT
            ..self.view.byte_offset + end * T::BYTES_PER_ELEMENT];
        if !bytes.is_empty() {
            value.write_bytes(&mut bytes[..T::BYTES_PER_ELEMENT]);
            let mut filled = T::BYTES_PER_ELEMENT;
            while filled < bytes.len() {
                let count = filled.min(bytes.len() - filled);
                bytes.copy_within(..count, filled);
                filled += count;
            }
        }
        self.clone()
    }

    pub fn fill_all(&self, value: f64) -> Self {
        self.fill(value, 0.0, None)
    }

    pub fn fill_from(&self, value: f64, start: f64) -> Self {
        self.fill(value, start, None)
    }

    pub fn fill_to(&self, value: f64, start: f64, end: f64) -> Self {
        self.fill(value, start, Some(end))
    }

    pub fn includes(&self, search: f64, from_index: f64) -> bool {
        let start = normalize_index(from_index, self.view.length);
        (start..self.view.length).any(|index| {
            self.get_usize(index)
                .is_some_and(|value| same_value_zero_number(value.to_number(), search))
        })
    }

    pub fn includes_from_start(&self, search: f64) -> bool {
        self.includes(search, 0.0)
    }

    pub fn index_of(&self, search: f64, from_index: f64) -> isize {
        let start = normalize_index(from_index, self.view.length);
        (start..self.view.length)
            .find(|index| {
                self.get_usize(*index)
                    .is_some_and(|value| value.to_number() == search)
            })
            .map_or(-1, |index| index as isize)
    }

    pub fn index_of_from_start(&self, search: f64) -> isize {
        self.index_of(search, 0.0)
    }

    pub fn join(&self, separator: &str) -> String {
        (0..self.view.length)
            .map(|index| {
                let value = self.get_usize(index).unwrap_or_default();
                value.to_string()
            })
            .collect::<Vec<_>>()
            .join(separator)
    }

    pub fn join_default(&self) -> String {
        self.join(",")
    }

    pub fn reverse(&self) -> Self {
        for left in 0..self.view.length / 2 {
            let right = self.view.length - left - 1;
            let left_value = self.get_usize(left).unwrap_or_default();
            let right_value = self.get_usize(right).unwrap_or_default();
            self.set_usize(left, right_value);
            self.set_usize(right, left_value);
        }
        self.clone()
    }

    pub fn set_from_array(&self, source: &JsArray<f64>, offset: f64) -> JsResult<()> {
        self.set_from_numbers(source.iter_values(), offset)
    }

    pub fn set_from_array_default(&self, source: &JsArray<f64>) -> JsResult<()> {
        self.set_from_array(source, 0.0)
    }

    pub fn set_from_fixed_array<const LENGTH: usize>(
        &self,
        source: &[f64; LENGTH],
        offset: f64,
    ) -> JsResult<()> {
        self.set_from_numbers(source.iter().copied(), offset)
    }

    pub fn set_from_typed_array<U: TypedElement + ConvertElement<T>>(
        &self,
        source: &TypedArray<U>,
        offset: f64,
    ) -> JsResult<()> {
        if T::KIND == U::KIND && T::BYTES_PER_ELEMENT == U::BYTES_PER_ELEMENT {
            let offset = to_index(offset)?;
            if offset
                .checked_add(source.view.length)
                .is_none_or(|end| end > self.view.length)
            {
                return Err(range_error("typed array set source out of bounds"));
            }
            self.view.buffer.copy_bytes_from(
                self.view.byte_offset + offset * T::BYTES_PER_ELEMENT,
                &source.view.buffer,
                source.view.byte_offset
                    ..source.view.byte_offset + source.view.length * U::BYTES_PER_ELEMENT,
            );
            return Ok(());
        }
        let offset = to_index(offset)?;
        if offset
            .checked_add(source.len())
            .is_none_or(|end| end > self.len())
        {
            return Err(range_error("typed array set source out of bounds"));
        }
        let target_start = self.view.byte_offset + offset * T::BYTES_PER_ELEMENT;
        let target_end = target_start + source.len() * T::BYTES_PER_ELEMENT;
        let source_end = source.view.byte_offset + source.len() * U::BYTES_PER_ELEMENT;
        let shared = self.view.buffer.shares_storage(&source.view.buffer);
        let overlaps = shared && target_start < source_end && source.view.byte_offset < target_end;
        if overlaps {
            let snapshot: Vec<T> = source.with_bytes(|bytes| {
                bytes
                    .chunks_exact(U::BYTES_PER_ELEMENT)
                    .map(|element| U::read_bytes(element).convert_element())
                    .collect()
            });
            let mut target = self.view.buffer.as_mut_bytes();
            for (value, element) in snapshot
                .into_iter()
                .zip(target[target_start..target_end].chunks_exact_mut(T::BYTES_PER_ELEMENT))
            {
                value.write_bytes(element);
            }
        } else if shared {
            let mut bytes = self.view.buffer.as_mut_bytes();
            if target_end <= source.view.byte_offset {
                let (target, input) = bytes.split_at_mut(source.view.byte_offset);
                Self::copy_converted::<U>(
                    &input[..source_end - source.view.byte_offset],
                    &mut target[target_start..target_end],
                );
            } else {
                let (input, target) = bytes.split_at_mut(target_start);
                Self::copy_converted::<U>(
                    &input[source.view.byte_offset..source_end],
                    &mut target[..target_end - target_start],
                );
            }
        } else {
            let input = source.view.buffer.as_bytes();
            let mut target = self.view.buffer.as_mut_bytes();
            Self::copy_converted::<U>(
                &input[source.view.byte_offset..source_end],
                &mut target[target_start..target_end],
            );
        }
        Ok(())
    }

    fn copy_converted<U: TypedElement + ConvertElement<T>>(source: &[u8], target: &mut [u8]) {
        for (input, output) in source
            .chunks_exact(U::BYTES_PER_ELEMENT)
            .zip(target.chunks_exact_mut(T::BYTES_PER_ELEMENT))
        {
            let value: T = U::read_bytes(input).convert_element();
            value.write_bytes(output);
        }
    }

    pub fn slice(&self, start: f64, end: Option<f64>) -> Self {
        let (start, end) = normalized_range(self.view.length, start, end);
        let result = Self::with_length(end - start).expect("normalized typed array length");
        result.view.buffer.copy_bytes_from(
            0,
            &self.view.buffer,
            self.view.byte_offset + start * T::BYTES_PER_ELEMENT
                ..self.view.byte_offset + end * T::BYTES_PER_ELEMENT,
        );
        result
    }

    pub fn slice_all(&self) -> Self {
        self.slice(0.0, None)
    }

    pub fn slice_from(&self, start: f64) -> Self {
        self.slice(start, None)
    }

    pub fn slice_to(&self, start: f64, end: f64) -> Self {
        self.slice(start, Some(end))
    }

    pub fn subarray(&self, start: f64, end: Option<f64>) -> Self {
        let (start, end) = normalized_range(self.view.length, start, end);
        Self::from_view(
            self.view.buffer.clone(),
            self.view.byte_offset + start * T::BYTES_PER_ELEMENT,
            end - start,
        )
    }

    pub fn subarray_all(&self) -> Self {
        self.subarray(0.0, None)
    }

    pub fn subarray_from(&self, start: f64) -> Self {
        self.subarray(start, None)
    }

    pub fn subarray_to(&self, start: f64, end: f64) -> Self {
        self.subarray(start, Some(end))
    }

    pub fn sort_default(&self) -> Self {
        let mut values = self.values();
        values.sort_by(|left, right| left.compare(*right));
        self.replace_values(values);
        self.clone()
    }

    pub fn sort_by(&self, mut compare: impl FnMut(T::Value, T::Value) -> f64) -> Self {
        let mut values = self.values();
        values.sort_by(|left, right| {
            let order = compare(left.into_value(), right.into_value());
            if order.is_nan() || order == 0.0 {
                Ordering::Equal
            } else if order < 0.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        self.replace_values(values);
        self.clone()
    }

    pub fn try_sort_by(
        &self,
        mut compare: impl FnMut(T::Value, T::Value) -> tsonic_rust_runtime::TsonicResult<f64>,
    ) -> tsonic_rust_runtime::TsonicResult<Self> {
        let mut values = self.values();
        let mut failure = None;
        values.sort_by(|left, right| match compare(left.into_value(), right.into_value()) {
            Ok(order) if order < 0.0 => Ordering::Less,
            Ok(order) if order > 0.0 => Ordering::Greater,
            Ok(_) => Ordering::Equal,
            Err(error) => {
                failure = Some(error);
                Ordering::Equal
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        self.replace_values(values);
        Ok(self.clone())
    }

    fn from_view(buffer: ArrayBuffer, byte_offset: usize, length: usize) -> Self {
        Self {
            view: Rc::new(TypedArrayView {
                buffer,
                byte_offset,
                length,
                identity: ObjectIdentity::new(),
            }),
            element: std::marker::PhantomData,
        }
    }

    pub(crate) fn get_usize(&self, index: usize) -> Option<T> {
        let (start, end) = self.byte_range(index)?;
        let bytes = self.view.buffer.as_bytes();
        Some(T::read_bytes(&bytes[start..end]))
    }

    fn set_usize(&self, index: usize, value: T) {
        let Some((start, end)) = self.byte_range(index) else {
            return;
        };
        value.write_bytes(&mut self.view.buffer.as_mut_bytes()[start..end]);
    }

    fn set_from_numbers(&self, source: impl IntoIterator<Item = f64>, offset: f64) -> JsResult<()> {
        let offset = to_index(offset)?;
        let values: Vec<f64> = source.into_iter().collect();
        if offset
            .checked_add(values.len())
            .is_none_or(|end| end > self.view.length)
        {
            return Err(range_error("typed array set source out of bounds"));
        }
        for (index, value) in values.into_iter().enumerate() {
            self.set_usize(offset + index, T::from_number(value));
        }
        Ok(())
    }

    fn values(&self) -> Vec<T> {
        (0..self.view.length)
            .map(|index| self.get_usize(index).unwrap_or_default())
            .collect()
    }

    fn replace_values(&self, values: Vec<T>) {
        for (index, value) in values.into_iter().enumerate() {
            self.set_usize(index, value);
        }
    }

    fn byte_range(&self, index: usize) -> Option<(usize, usize)> {
        if index >= self.view.length {
            return None;
        }
        let start = self.view.byte_offset + index * T::BYTES_PER_ELEMENT;
        Some((start, start + T::BYTES_PER_ELEMENT))
    }
}

impl<T: TypedElement> PartialEq for TypedArray<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.view, &other.view)
    }
}

impl<T: TypedElement> Eq for TypedArray<T> {}

impl<T: TypedElement> JsSameValueZero for TypedArray<T> {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: TypedElement> JsHash for TypedArray<T> {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.view) as usize)
    }
}

impl<T: TypedElement> JsStrictEqual for TypedArray<T> {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: TypedElement> ObjectIdentityCarrier for TypedArray<T> {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.view.identity
    }
}

fn normalized_range(length: usize, start: f64, end: Option<f64>) -> (usize, usize) {
    let start = normalize_index(start, length);
    let end = normalize_index(end.unwrap_or(length as f64), length);
    (start, end.max(start))
}

fn same_value_zero_number(left: f64, right: f64) -> bool {
    left == right || left.is_nan() && right.is_nan()
}
