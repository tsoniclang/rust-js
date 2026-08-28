//! JavaScript typed-array carriers with exact numeric conversion, copy `slice`,
//! and shared-backing-store `subarray` semantics.

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

use crate::array::JsArray;
use crate::array_buffer::{normalize_index, to_index, ArrayBuffer};
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl fmt::Display for TypedArrayKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Int8 => "Int8",
            Self::Uint8 => "Uint8",
            Self::Uint8Clamped => "Uint8Clamped",
            Self::Int16 => "Int16",
            Self::Uint16 => "Uint16",
            Self::Int32 => "Int32",
            Self::Uint32 => "Uint32",
            Self::Float32 => "Float32",
            Self::Float64 => "Float64",
        })
    }
}

pub trait TypedElement: Copy + Default {
    const KIND: TypedArrayKind;
    const BYTES_PER_ELEMENT: usize;

    fn from_number(value: f64) -> Self;
    fn to_number(self) -> f64;
    fn write_bytes(self, output: &mut [u8]);
    fn read_bytes(bytes: &[u8]) -> Self;
}

macro_rules! integer_element {
    ($type:ty, $kind:expr, $bits:expr, $signed:expr) => {
        impl TypedElement for $type {
            const KIND: TypedArrayKind = $kind;
            const BYTES_PER_ELEMENT: usize = std::mem::size_of::<$type>();

            fn from_number(value: f64) -> Self {
                integer_number(value, $bits, $signed) as $type
            }

            fn to_number(self) -> f64 {
                self as f64
            }

            fn write_bytes(self, output: &mut [u8]) {
                output.copy_from_slice(&self.to_le_bytes());
            }

            fn read_bytes(bytes: &[u8]) -> Self {
                let mut slot = [0_u8; std::mem::size_of::<$type>()];
                slot.copy_from_slice(bytes);
                <$type>::from_le_bytes(slot)
            }
        }
    };
}

integer_element!(i8, TypedArrayKind::Int8, 8, true);
integer_element!(u8, TypedArrayKind::Uint8, 8, false);
integer_element!(i16, TypedArrayKind::Int16, 16, true);
integer_element!(u16, TypedArrayKind::Uint16, 16, false);
integer_element!(i32, TypedArrayKind::Int32, 32, true);
integer_element!(u32, TypedArrayKind::Uint32, 32, false);

impl TypedElement for f32 {
    const KIND: TypedArrayKind = TypedArrayKind::Float32;
    const BYTES_PER_ELEMENT: usize = 4;

    fn from_number(value: f64) -> Self {
        value as f32
    }

    fn to_number(self) -> f64 {
        self as f64
    }

    fn write_bytes(self, output: &mut [u8]) {
        output.copy_from_slice(&self.to_le_bytes());
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        let mut slot = [0_u8; 4];
        slot.copy_from_slice(bytes);
        Self::from_le_bytes(slot)
    }
}

impl TypedElement for f64 {
    const KIND: TypedArrayKind = TypedArrayKind::Float64;
    const BYTES_PER_ELEMENT: usize = 8;

    fn from_number(value: f64) -> Self {
        value
    }

    fn to_number(self) -> f64 {
        self
    }

    fn write_bytes(self, output: &mut [u8]) {
        output.copy_from_slice(&self.to_le_bytes());
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        let mut slot = [0_u8; 8];
        slot.copy_from_slice(bytes);
        Self::from_le_bytes(slot)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClampedU8(pub u8);

impl TypedElement for ClampedU8 {
    const KIND: TypedArrayKind = TypedArrayKind::Uint8Clamped;
    const BYTES_PER_ELEMENT: usize = 1;

    fn from_number(value: f64) -> Self {
        Self(to_uint8_clamp(value))
    }

    fn to_number(self) -> f64 {
        self.0 as f64
    }

    fn write_bytes(self, output: &mut [u8]) {
        output[0] = self.0;
    }

    fn read_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0])
    }
}

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

impl<T: TypedElement> TypedArray<T> {
    pub const BYTES_PER_ELEMENT: f64 = T::BYTES_PER_ELEMENT as f64;

    pub fn new(length: f64) -> JsResult<Self> {
        let length = to_index(length)?;
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

    pub fn from_fixed_array<const LENGTH: usize>(values: &[f64; LENGTH]) -> JsResult<Self> {
        Self::from_numbers(values.iter().copied())
    }

    pub fn from_vec(values: Vec<f64>) -> JsResult<Self> {
        Self::from_numbers(values)
    }

    pub fn from_numbers(values: impl IntoIterator<Item = f64>) -> JsResult<Self> {
        let values: Vec<f64> = values.into_iter().collect();
        let result = Self::new(values.len() as f64)?;
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
        let buffer_length = buffer.byte_length_usize();
        if byte_offset > buffer_length || byte_offset % T::BYTES_PER_ELEMENT != 0 {
            return Err(range_error("typed array byte offset is invalid"));
        }
        let available = buffer_length - byte_offset;
        let length = match length {
            Some(value) => to_index(value)?,
            None if available % T::BYTES_PER_ELEMENT == 0 => available / T::BYTES_PER_ELEMENT,
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

    pub fn bytes_per_element(&self) -> f64 {
        Self::BYTES_PER_ELEMENT
    }

    pub fn byte_length(&self) -> f64 {
        (self.view.length * T::BYTES_PER_ELEMENT) as f64
    }

    pub fn byte_offset(&self) -> f64 {
        self.view.byte_offset as f64
    }

    pub fn length(&self) -> f64 {
        self.view.length as f64
    }

    pub fn at(&self, index: f64) -> Option<f64> {
        let integer = if index.is_nan() { 0.0 } else { index.trunc() };
        let normalized = if integer < 0.0 {
            self.view.length as f64 + integer
        } else {
            integer
        };
        if normalized < 0.0 || normalized >= self.view.length as f64 {
            None
        } else {
            self.get_usize(normalized as usize).map(TypedElement::to_number)
        }
    }

    pub fn get_number(&self, index: f64) -> Option<f64> {
        if !index.is_finite() || index < 0.0 || index.fract() != 0.0 {
            return None;
        }
        self.get_usize(index as usize).map(TypedElement::to_number)
    }

    pub fn set_number(&self, index: f64, value: f64) {
        if index.is_finite() && index >= 0.0 && index.fract() == 0.0 {
            self.set_usize(index as usize, T::from_number(value));
        }
    }

    pub fn fill(&self, value: f64, start: f64, end: Option<f64>) -> Self {
        let (start, end) = normalized_range(self.view.length, start, end);
        let value = T::from_number(value);
        for index in start..end {
            self.set_usize(index, value);
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

    pub fn index_of(&self, search: f64, from_index: f64) -> f64 {
        let start = normalize_index(from_index, self.view.length);
        (start..self.view.length)
            .find(|index| self.get_usize(*index).is_some_and(|value| value.to_number() == search))
            .map_or(-1.0, |index| index as f64)
    }

    pub fn index_of_from_start(&self, search: f64) -> f64 {
        self.index_of(search, 0.0)
    }

    pub fn join(&self, separator: &str) -> String {
        (0..self.view.length)
            .map(|index| {
                let value = self.get_usize(index).unwrap_or_default().to_number();
                ryu_js::Buffer::new().format(value).to_owned()
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

    pub fn set_from_typed_array<U: TypedElement>(
        &self,
        source: &TypedArray<U>,
        offset: f64,
    ) -> JsResult<()> {
        self.set_from_numbers(
            (0..source.view.length)
                .map(|index| source.get_usize(index).unwrap_or_default().to_number()),
            offset,
        )
    }

    pub fn slice(&self, start: f64, end: Option<f64>) -> Self {
        let (start, end) = normalized_range(self.view.length, start, end);
        let result = Self::new((end - start) as f64).expect("normalized typed array length");
        for (target, source) in (start..end).enumerate() {
            result.set_usize(target, self.get_usize(source).unwrap_or_default());
        }
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
        values.sort_by(|left, right| left.total_cmp(right));
        self.replace_numbers(values);
        self.clone()
    }

    pub fn sort_by(&self, mut compare: impl FnMut(f64, f64) -> f64) -> Self {
        let mut values = self.values();
        values.sort_by(|left, right| {
            let order = compare(*left, *right);
            if order.is_nan() || order == 0.0 {
                Ordering::Equal
            } else if order < 0.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        self.replace_numbers(values);
        self.clone()
    }

    pub fn try_sort_by(
        &self,
        mut compare: impl FnMut(f64, f64) -> tsonic_rust_runtime::TsonicResult<f64>,
    ) -> tsonic_rust_runtime::TsonicResult<Self> {
        let mut values = self.values();
        let mut failure = None;
        values.sort_by(|left, right| match compare(*left, *right) {
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
        self.replace_numbers(values);
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

    fn get_usize(&self, index: usize) -> Option<T> {
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

    fn set_from_numbers(
        &self,
        source: impl IntoIterator<Item = f64>,
        offset: f64,
    ) -> JsResult<()> {
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

    fn values(&self) -> Vec<f64> {
        (0..self.view.length)
            .map(|index| self.get_usize(index).unwrap_or_default().to_number())
            .collect()
    }

    fn replace_numbers(&self, values: Vec<f64>) {
        for (index, value) in values.into_iter().enumerate() {
            self.set_usize(index, T::from_number(value));
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

pub(crate) fn integer_number(value: f64, bits: u32, signed: bool) -> i128 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let modulus = 2_f64.powi(bits as i32);
    let unsigned = value.trunc().rem_euclid(modulus);
    if signed && unsigned >= modulus / 2.0 {
        (unsigned - modulus) as i128
    } else {
        unsigned as i128
    }
}

fn to_uint8_clamp(value: f64) -> u8 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    if value >= 255.0 {
        return 255;
    }
    let floor = value.floor();
    let fraction = value - floor;
    if fraction > 0.5 || fraction == 0.5 && (floor as u8) % 2 == 1 {
        floor as u8 + 1
    } else {
        floor as u8
    }
}

fn same_value_zero_number(left: f64, right: f64) -> bool {
    left == right || left.is_nan() && right.is_nan()
}
