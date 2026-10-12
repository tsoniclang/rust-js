use std::cell::{Ref, RefMut};
use std::ops::{Deref, DerefMut, Range};
use std::sync::MutexGuard;

use super::shared::SharedState;

pub struct BufferBytes<'a>(pub(crate) ByteReadGuard<'a>);
pub struct BufferBytesMut<'a>(pub(crate) ByteWriteGuard<'a>);

#[derive(Debug)]
pub struct OwnedBufferBytes {
    bytes: Vec<u8>,
    range: Range<usize>,
}

impl OwnedBufferBytes {
    pub(crate) fn from_range(bytes: Vec<u8>, range: Range<usize>) -> Self {
        assert!(range.start <= range.end && range.end <= bytes.len());
        Self { bytes, range }
    }

    pub(crate) fn copy(bytes: &[u8]) -> Self {
        Self::from_range(bytes.to_vec(), 0..bytes.len())
    }
}

impl Deref for OwnedBufferBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes[self.range.clone()]
    }
}

pub(crate) enum ByteReadGuard<'a> {
    Ordinary(Ref<'a, [u8]>),
    Shared(MutexGuard<'a, SharedState>),
}

pub(crate) enum ByteWriteGuard<'a> {
    Ordinary(RefMut<'a, [u8]>),
    Shared(MutexGuard<'a, SharedState>),
}

impl Deref for BufferBytes<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match &self.0 {
            ByteReadGuard::Ordinary(bytes) => bytes,
            ByteReadGuard::Shared(state) => &state.bytes,
        }
    }
}

impl Deref for BufferBytesMut<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match &self.0 {
            ByteWriteGuard::Ordinary(bytes) => bytes,
            ByteWriteGuard::Shared(state) => &state.bytes,
        }
    }
}

impl DerefMut for BufferBytesMut<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        match &mut self.0 {
            ByteWriteGuard::Ordinary(bytes) => bytes,
            ByteWriteGuard::Shared(state) => &mut state.bytes,
        }
    }
}
