use std::sync::Mutex;

use crate::array_buffer::{to_index, SharedBufferStorage};
use crate::errors::{range_error, type_error, JsResult};
use crate::typed_array::{Int32Array, TypedElement};

pub(crate) static ATOMIC_ORDER: Mutex<()> = Mutex::new(());

fn address(array: &Int32Array, index: f64) -> JsResult<usize> {
    let index = to_index(index)?;
    if index >= array.len() {
        return Err(range_error("atomic index is outside the typed array"));
    }
    Ok(array.byte_offset() as usize + index * 4)
}

pub fn wait(array: &Int32Array, index: f64, expected: f64, timeout: f64) -> JsResult<String> {
    let offset = address(array, index)?;
    let storage = array
        .buffer()
        .shared_storage()
        .ok_or_else(|| type_error("Atomics.wait requires shared backing"))?;
    Ok(
        SharedBufferStorage::wait_i32(&storage, offset, i32::from_number(expected), timeout)?
            .into(),
    )
}

pub fn wait_forever(array: &Int32Array, index: f64, expected: f64) -> JsResult<String> {
    wait(array, index, expected, f64::INFINITY)
}

pub fn notify(array: &Int32Array, index: f64, count: f64) -> JsResult<f64> {
    let offset = address(array, index)?;
    Ok(array
        .buffer()
        .shared_storage()
        .map_or(0, |storage| storage.notify(offset, count)) as f64)
}

pub fn notify_all(array: &Int32Array, index: f64) -> JsResult<f64> {
    notify(array, index, f64::INFINITY)
}

pub fn load(array: &Int32Array, index: f64) -> JsResult<f64> {
    let offset = address(array, index)?;
    let _order = ATOMIC_ORDER.lock().expect("atomic order lock poisoned");
    let buffer = array.buffer();
    let bytes = buffer.as_bytes();
    Ok(i32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked Int32 width"),
    ) as f64)
}

pub fn store(array: &Int32Array, index: f64, value: f64) -> JsResult<f64> {
    let offset = address(array, index)?;
    let integer = if value.is_nan() { 0.0 } else { value.trunc() };
    let _order = ATOMIC_ORDER.lock().expect("atomic order lock poisoned");
    let buffer = array.buffer();
    let mut bytes = buffer.as_mut_bytes();
    bytes[offset..offset + 4].copy_from_slice(&i32::from_number(integer).to_le_bytes());
    Ok(integer)
}
