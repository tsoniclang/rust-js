//! Closed ArrayBuffer carrier.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayBuffer {
    bytes: Rc<RefCell<Vec<u8>>>,
}

impl ArrayBuffer {
    pub fn new(byte_length: usize) -> Self {
        Self {
            bytes: Rc::new(RefCell::new(vec![0_u8; byte_length])),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Rc::new(RefCell::new(bytes)),
        }
    }

    pub fn byte_length(&self) -> usize {
        self.bytes.borrow().len()
    }

    pub fn as_bytes(&self) -> Ref<'_, [u8]> {
        Ref::map(self.bytes.borrow(), Vec::as_slice)
    }

    pub fn as_mut_bytes(&self) -> RefMut<'_, [u8]> {
        RefMut::map(self.bytes.borrow_mut(), Vec::as_mut_slice)
    }

    pub fn slice(&self, start: isize, end: Option<isize>) -> Self {
        let bytes = self.bytes.borrow();
        let max = bytes.len() as isize;
        let s = normalize_index(start, max);
        let e = normalize_index(end.unwrap_or(max), max);
        Self::from_bytes(if e <= s {
            Vec::new()
        } else {
            bytes[s..e].to_vec()
        })
    }

    pub(crate) fn shared_bytes(&self) -> Rc<RefCell<Vec<u8>>> {
        Rc::clone(&self.bytes)
    }
}

fn normalize_index(value: isize, max: isize) -> usize {
    let clamped = if value < 0 {
        max.saturating_add(value)
    } else {
        value
    }
    .clamp(0, max);
    clamped as usize
}
