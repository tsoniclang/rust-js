use std::sync::Mutex;

use crate::array_buffer::{to_index, SharedBufferStorage};
use crate::errors::{range_error, type_error, JsResult};
use crate::native_integer::Integer32;
use crate::numeric::IndexInput;
use crate::typed_array::Int32Array;

pub(crate) static ATOMIC_ORDER: Mutex<()> = Mutex::new(());

fn address(array: &Int32Array, index: impl IndexInput) -> JsResult<usize> {
    let index = to_index(index)?;
    if index >= array.len() {
        return Err(range_error("atomic index is outside the typed array"));
    }
    Ok(array.byte_offset() + index * 4)
}

pub fn wait(
    array: &Int32Array,
    index: impl IndexInput,
    expected: impl Integer32,
    timeout: f64,
) -> JsResult<String> {
    let offset = address(array, index)?;
    let storage = array
        .buffer()
        .shared_storage()
        .ok_or_else(|| type_error("Atomics.wait requires shared backing"))?;
    Ok(
        SharedBufferStorage::wait_i32(&storage, offset, expected.integer32() as i32, timeout)?
            .into(),
    )
}

pub fn wait_forever(
    array: &Int32Array,
    index: impl IndexInput,
    expected: impl Integer32,
) -> JsResult<String> {
    wait(array, index, expected, f64::INFINITY)
}

pub fn notify(
    array: &Int32Array,
    index: impl IndexInput,
    count: impl IndexInput,
) -> JsResult<usize> {
    let offset = address(array, index)?;
    Ok(array.buffer().shared_storage().map_or(0, |storage| {
        storage.notify(offset, count.positive_index(usize::MAX))
    }))
}

pub fn notify_all(array: &Int32Array, index: impl IndexInput) -> JsResult<usize> {
    notify(array, index, usize::MAX)
}

pub fn load(array: &Int32Array, index: impl IndexInput) -> JsResult<i32> {
    let offset = address(array, index)?;
    let _order = ATOMIC_ORDER.lock().expect("atomic order lock poisoned");
    let buffer = array.buffer();
    let bytes = buffer.as_bytes();
    Ok(i32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked Int32 width"),
    ))
}

pub fn store<Value: StoreValue>(
    array: &Int32Array,
    index: impl IndexInput,
    value: Value,
) -> JsResult<Value> {
    let offset = address(array, index)?;
    let integer = value.normalize();
    let _order = ATOMIC_ORDER.lock().expect("atomic order lock poisoned");
    let buffer = array.buffer();
    let mut bytes = buffer.as_mut_bytes();
    bytes[offset..offset + 4].copy_from_slice(&integer.integer32().to_le_bytes());
    Ok(integer)
}

pub trait StoreValue: Integer32 + Copy {
    fn normalize(self) -> Self;
}

macro_rules! native_store_values {
    ($($native:ty),+ $(,)?) => {$(
        impl StoreValue for $native {
            #[inline]
            fn normalize(self) -> Self { self }
        }
    )+};
}

native_store_values!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

impl StoreValue for f32 {
    #[inline]
    fn normalize(self) -> Self {
        if self.is_nan() {
            0.0
        } else {
            self.trunc()
        }
    }
}

impl StoreValue for f64 {
    #[inline]
    fn normalize(self) -> Self {
        if self.is_nan() {
            0.0
        } else {
            self.trunc()
        }
    }
}
