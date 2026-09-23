//! Closed ArrayBuffer carrier.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};

mod bytes;
mod shared;
pub use bytes::{BufferBytes, BufferBytesMut};
use bytes::{ByteReadGuard, ByteWriteGuard};
pub use shared::SharedBufferStorage;

#[derive(Debug, Clone)]
enum BufferStorage {
    Ordinary(Rc<RefCell<Vec<u8>>>),
    Shared(Arc<SharedBufferStorage>),
}

#[derive(Debug, Clone)]
pub struct ArrayBuffer {
    storage: BufferStorage,
    identity: ObjectIdentity,
}

impl PartialEq for ArrayBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
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
        hash_identity(self.identity.key())
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
            storage: BufferStorage::Ordinary(Rc::new(RefCell::new(vec![0_u8; byte_length]))),
            identity: ObjectIdentity::new(),
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            storage: BufferStorage::Ordinary(Rc::new(RefCell::new(bytes))),
            identity: ObjectIdentity::new(),
        }
    }

    pub fn byte_length(&self) -> f64 {
        self.byte_length_usize() as f64
    }

    pub fn new_shared(byte_length: f64) -> JsResult<Self> {
        let byte_length = to_index(byte_length)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(byte_length)
            .map_err(|_| range_error("shared buffer allocation exceeds native capacity"))?;
        bytes.resize(byte_length, 0);
        Ok(Self::from_shared_storage(Arc::new(
            SharedBufferStorage::new(bytes),
        )))
    }

    pub fn from_shared_storage(storage: Arc<SharedBufferStorage>) -> Self {
        Self {
            storage: BufferStorage::Shared(storage),
            identity: ObjectIdentity::new(),
        }
    }

    pub fn shared_storage(&self) -> Option<Arc<SharedBufferStorage>> {
        match &self.storage {
            BufferStorage::Shared(storage) => Some(Arc::clone(storage)),
            BufferStorage::Ordinary(_) => None,
        }
    }

    pub fn as_bytes(&self) -> BufferBytes<'_> {
        BufferBytes(match &self.storage {
            BufferStorage::Ordinary(bytes) => {
                ByteReadGuard::Ordinary(Ref::map(bytes.borrow(), Vec::as_slice))
            }
            BufferStorage::Shared(storage) => ByteReadGuard::Shared(storage.lock()),
        })
    }

    pub fn as_mut_bytes(&self) -> BufferBytesMut<'_> {
        BufferBytesMut(match &self.storage {
            BufferStorage::Ordinary(bytes) => {
                ByteWriteGuard::Ordinary(RefMut::map(bytes.borrow_mut(), Vec::as_mut_slice))
            }
            BufferStorage::Shared(storage) => ByteWriteGuard::Shared(storage.lock()),
        })
    }

    fn storage_key(&self) -> (u8, usize) {
        match &self.storage {
            BufferStorage::Ordinary(storage) => (0, Rc::as_ptr(storage) as usize),
            BufferStorage::Shared(storage) => (1, Arc::as_ptr(storage) as usize),
        }
    }

    pub fn with_byte_ranges<R>(
        &self,
        left: std::ops::Range<usize>,
        other: &Self,
        right: std::ops::Range<usize>,
        operation: impl FnOnce(&[u8], &[u8]) -> R,
    ) -> R {
        let left_key = self.storage_key();
        let right_key = other.storage_key();
        if left_key == right_key {
            let bytes = self.as_bytes();
            operation(&bytes[left], &bytes[right])
        } else if left_key < right_key {
            let left_bytes = self.as_bytes();
            let right_bytes = other.as_bytes();
            operation(&left_bytes[left], &right_bytes[right])
        } else {
            let right_bytes = other.as_bytes();
            let left_bytes = self.as_bytes();
            operation(&left_bytes[left], &right_bytes[right])
        }
    }

    pub fn copy_bytes_from(
        &self,
        destination: usize,
        source: &Self,
        range: std::ops::Range<usize>,
    ) {
        let destination_key = self.storage_key();
        let source_key = source.storage_key();
        if destination_key == source_key {
            self.as_mut_bytes().copy_within(range, destination);
        } else if destination_key < source_key {
            let mut output = self.as_mut_bytes();
            let input = source.as_bytes();
            output[destination..destination + range.len()].copy_from_slice(&input[range]);
        } else {
            let input = source.as_bytes();
            let mut output = self.as_mut_bytes();
            output[destination..destination + range.len()].copy_from_slice(&input[range]);
        }
    }

    pub fn slice(&self, start: f64, end: Option<f64>) -> Self {
        let bytes = self.as_bytes();
        let max = bytes.len();
        let s = normalize_index(start, max);
        let e = normalize_index(end.unwrap_or(max as f64), max);
        let copied = if e <= s {
            Vec::new()
        } else {
            bytes[s..e].to_vec()
        };
        match &self.storage {
            BufferStorage::Ordinary(_) => Self::from_bytes(copied),
            BufferStorage::Shared(_) => {
                Self::from_shared_storage(Arc::new(SharedBufferStorage::new(copied)))
            }
        }
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
        self.as_bytes().len()
    }
}

impl ObjectIdentityCarrier for ArrayBuffer {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}

pub(crate) fn to_index(value: f64) -> JsResult<usize> {
    crate::coercion::native_length(value)
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
