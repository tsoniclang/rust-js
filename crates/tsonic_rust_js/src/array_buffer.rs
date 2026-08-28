//! Closed ArrayBuffer carrier.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};

#[derive(Debug, Clone)]
pub struct ArrayBuffer {
    bytes: Rc<RefCell<Vec<u8>>>,
    identity: ObjectIdentity,
}

impl PartialEq for ArrayBuffer {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.bytes, &other.bytes)
    }
}

impl Eq for ArrayBuffer {}

impl JsSameValueZero for ArrayBuffer {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for ArrayBuffer {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.bytes) as usize)
    }
}

impl JsStrictEqual for ArrayBuffer {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl ArrayBuffer {
    pub fn new(byte_length: f64) -> JsResult<Self> {
        let byte_length = to_index(byte_length)?;
        Ok(Self {
            bytes: Rc::new(RefCell::new(vec![0_u8; byte_length])),
            identity: ObjectIdentity::new(),
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Rc::new(RefCell::new(bytes)),
            identity: ObjectIdentity::new(),
        }
    }

    pub fn byte_length(&self) -> f64 {
        self.byte_length_usize() as f64
    }

    pub fn as_bytes(&self) -> Ref<'_, [u8]> {
        Ref::map(self.bytes.borrow(), Vec::as_slice)
    }

    pub fn as_mut_bytes(&self) -> RefMut<'_, [u8]> {
        RefMut::map(self.bytes.borrow_mut(), Vec::as_mut_slice)
    }

    pub fn slice(&self, start: f64, end: Option<f64>) -> Self {
        let bytes = self.bytes.borrow();
        let max = bytes.len();
        let s = normalize_index(start, max);
        let e = normalize_index(end.unwrap_or(max as f64), max);
        Self::from_bytes(if e <= s {
            Vec::new()
        } else {
            bytes[s..e].to_vec()
        })
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

    pub(crate) fn byte_length_usize(&self) -> usize {
        self.bytes.borrow().len()
    }
}

impl ObjectIdentityCarrier for ArrayBuffer {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

pub(crate) fn to_index(value: f64) -> JsResult<usize> {
    if !value.is_finite() || value < 0.0 || value.trunc() > usize::MAX as f64 {
        return Err(range_error(
            "ArrayBuffer index is outside the supported range",
        ));
    }
    Ok(value.trunc() as usize)
}

pub(crate) fn normalize_index(value: f64, max: usize) -> usize {
    let integer = if value.is_nan() { 0.0 } else { value.trunc() };
    let max = max as f64;
    let clamped = if integer < 0.0 {
        max + integer
    } else {
        integer
    }
    .clamp(0.0, max);
    clamped as usize
}
