use crate::array_buffer::ArrayBuffer;
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct DataViewState {
    bytes: Rc<RefCell<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct DataView {
    state: Rc<DataViewState>,
}

impl PartialEq for DataView {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for DataView {}

impl JsSameValueZero for DataView {
    fn same_value_zero(&self, other: &Self) -> bool {
        self == other
    }
}

impl JsHash for DataView {
    fn js_hash(&self) -> u64 {
        hash_identity(Rc::as_ptr(&self.state) as usize)
    }
}

impl JsStrictEqual for DataView {
    fn strict_equal(&self, other: &Self) -> bool {
        self == other
    }
}

impl DataView {
    pub fn new(buffer: ArrayBuffer) -> Self {
        Self {
            state: Rc::new(DataViewState {
                bytes: buffer.shared_bytes(),
            }),
        }
    }

    pub fn byte_length(&self) -> usize {
        self.state.bytes.borrow().len()
    }

    pub fn get_uint8(&self, offset: usize) -> JsResult<u8> {
        self.state
            .bytes
            .borrow()
            .get(offset)
            .copied()
            .ok_or_else(|| range_error("DataView offset out of bounds"))
    }

    pub fn set_uint8(&mut self, offset: usize, value: u8) -> JsResult<()> {
        let mut bytes = self.state.bytes.borrow_mut();
        let slot = bytes
            .get_mut(offset)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        *slot = value;
        Ok(())
    }

    pub fn get_int32(&self, offset: usize, little_endian: bool) -> JsResult<i32> {
        let bytes = self.read::<4>(offset)?;
        Ok(if little_endian {
            i32::from_le_bytes(bytes)
        } else {
            i32::from_be_bytes(bytes)
        })
    }

    pub fn set_int32(&mut self, offset: usize, value: i32, little_endian: bool) -> JsResult<()> {
        let bytes = if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        self.write(offset, &bytes)
    }

    pub fn get_float64(&self, offset: usize, little_endian: bool) -> JsResult<f64> {
        let bytes = self.read::<8>(offset)?;
        Ok(if little_endian {
            f64::from_le_bytes(bytes)
        } else {
            f64::from_be_bytes(bytes)
        })
    }

    pub fn set_float64(&mut self, offset: usize, value: f64, little_endian: bool) -> JsResult<()> {
        let bytes = if little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        self.write(offset, &bytes)
    }

    fn read<const LENGTH: usize>(&self, offset: usize) -> JsResult<[u8; LENGTH]> {
        let end = offset
            .checked_add(LENGTH)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        let bytes = self.state.bytes.borrow();
        let slice = bytes
            .get(offset..end)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        let mut result = [0_u8; LENGTH];
        result.copy_from_slice(slice);
        Ok(result)
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) -> JsResult<()> {
        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        let mut storage = self.state.bytes.borrow_mut();
        let target = storage
            .get_mut(offset..end)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        target.copy_from_slice(bytes);
        Ok(())
    }
}
