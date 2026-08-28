use crate::array_buffer::{to_index, ArrayBuffer};
use crate::equality::{hash_identity, JsHash, JsSameValueZero, JsStrictEqual};
use crate::errors::{range_error, JsResult};
use crate::typed_array::integer_number;
use std::rc::Rc;
use tsonic_rust_runtime::{ObjectIdentity, ObjectIdentityCarrier};

#[derive(Debug)]
struct DataViewState {
    buffer: ArrayBuffer,
    byte_offset: usize,
    byte_length: usize,
    identity: ObjectIdentity,
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
    pub fn from_buffer(buffer: ArrayBuffer) -> JsResult<Self> {
        Self::new(buffer, 0.0, None)
    }

    pub fn from_buffer_offset(buffer: ArrayBuffer, byte_offset: f64) -> JsResult<Self> {
        Self::new(buffer, byte_offset, None)
    }

    pub fn from_buffer_length(
        buffer: ArrayBuffer,
        byte_offset: f64,
        byte_length: f64,
    ) -> JsResult<Self> {
        Self::new(buffer, byte_offset, Some(byte_length))
    }

    pub fn new(buffer: ArrayBuffer, byte_offset: f64, byte_length: Option<f64>) -> JsResult<Self> {
        let byte_offset = to_index(byte_offset)?;
        let buffer_length = buffer.byte_length_usize();
        if byte_offset > buffer_length {
            return Err(range_error("DataView byte offset out of bounds"));
        }
        let byte_length = match byte_length {
            Some(value) => to_index(value)?,
            None => buffer_length - byte_offset,
        };
        if byte_offset.checked_add(byte_length).is_none_or(|end| end > buffer_length) {
            return Err(range_error("DataView byte length out of bounds"));
        }
        Ok(Self {
            state: Rc::new(DataViewState {
                buffer,
                byte_offset,
                byte_length,
                identity: ObjectIdentity::new(),
            }),
        })
    }

    pub fn buffer(&self) -> ArrayBuffer {
        self.state.buffer.clone()
    }

    pub fn byte_offset(&self) -> f64 {
        self.state.byte_offset as f64
    }

    pub fn byte_length(&self) -> f64 {
        self.state.byte_length as f64
    }

    pub fn get_int8(&self, offset: f64) -> JsResult<f64> {
        Ok(i8::from_ne_bytes(self.read::<1>(offset)?) as f64)
    }

    pub fn get_uint8(&self, offset: f64) -> JsResult<f64> {
        Ok(self.read::<1>(offset)?[0] as f64)
    }

    pub fn get_int16(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<2>(offset)?, little_endian, i16::from_le_bytes, i16::from_be_bytes) as f64)
    }

    pub fn get_uint16(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<2>(offset)?, little_endian, u16::from_le_bytes, u16::from_be_bytes) as f64)
    }

    pub fn get_int32(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<4>(offset)?, little_endian, i32::from_le_bytes, i32::from_be_bytes) as f64)
    }

    pub fn get_uint32(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<4>(offset)?, little_endian, u32::from_le_bytes, u32::from_be_bytes) as f64)
    }

    pub fn get_float32(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<4>(offset)?, little_endian, f32::from_le_bytes, f32::from_be_bytes) as f64)
    }

    pub fn get_float64(&self, offset: f64, little_endian: bool) -> JsResult<f64> {
        Ok(read_number(self.read::<8>(offset)?, little_endian, f64::from_le_bytes, f64::from_be_bytes))
    }

    pub fn set_int8(&self, offset: f64, value: f64) -> JsResult<()> {
        self.write(offset, &(integer_number(value, 8, true) as i8).to_ne_bytes())
    }

    pub fn set_uint8(&self, offset: f64, value: f64) -> JsResult<()> {
        self.write(offset, &(integer_number(value, 8, false) as u8).to_ne_bytes())
    }

    pub fn set_int16(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(
            offset,
            integer_number(value, 16, true) as i16,
            little_endian,
            i16::to_le_bytes,
            i16::to_be_bytes,
        )
    }

    pub fn set_uint16(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(
            offset,
            integer_number(value, 16, false) as u16,
            little_endian,
            u16::to_le_bytes,
            u16::to_be_bytes,
        )
    }

    pub fn set_int32(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(
            offset,
            integer_number(value, 32, true) as i32,
            little_endian,
            i32::to_le_bytes,
            i32::to_be_bytes,
        )
    }

    pub fn set_uint32(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(
            offset,
            integer_number(value, 32, false) as u32,
            little_endian,
            u32::to_le_bytes,
            u32::to_be_bytes,
        )
    }

    pub fn set_float32(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(offset, value as f32, little_endian, f32::to_le_bytes, f32::to_be_bytes)
    }

    pub fn set_float64(&self, offset: f64, value: f64, little_endian: bool) -> JsResult<()> {
        self.write_endian(offset, value, little_endian, f64::to_le_bytes, f64::to_be_bytes)
    }

    fn read<const LENGTH: usize>(&self, offset: f64) -> JsResult<[u8; LENGTH]> {
        let offset = to_index(offset)?;
        let end = offset
            .checked_add(LENGTH)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        if end > self.state.byte_length {
            return Err(range_error("DataView offset out of bounds"));
        }
        let start = self.state.byte_offset + offset;
        let end = self.state.byte_offset + end;
        let bytes = self.state.buffer.as_bytes();
        let slice = bytes
            .get(start..end)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        let mut result = [0_u8; LENGTH];
        result.copy_from_slice(slice);
        Ok(result)
    }

    fn write(&self, offset: f64, bytes: &[u8]) -> JsResult<()> {
        let offset = to_index(offset)?;
        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        if end > self.state.byte_length {
            return Err(range_error("DataView offset out of bounds"));
        }
        let start = self.state.byte_offset + offset;
        let end = self.state.byte_offset + end;
        let mut storage = self.state.buffer.as_mut_bytes();
        let target = storage
            .get_mut(start..end)
            .ok_or_else(|| range_error("DataView offset out of bounds"))?;
        target.copy_from_slice(bytes);
        Ok(())
    }

    fn write_endian<T, const LENGTH: usize>(
        &self,
        offset: f64,
        value: T,
        little_endian: bool,
        little: impl FnOnce(T) -> [u8; LENGTH],
        big: impl FnOnce(T) -> [u8; LENGTH],
    ) -> JsResult<()> {
        self.write(offset, &if little_endian { little(value) } else { big(value) })
    }
}

impl ObjectIdentityCarrier for DataView {
    fn object_identity(&self) -> &ObjectIdentity {
        &self.state.identity
    }
}

fn read_number<T, const LENGTH: usize>(
    bytes: [u8; LENGTH],
    little_endian: bool,
    little: impl FnOnce([u8; LENGTH]) -> T,
    big: impl FnOnce([u8; LENGTH]) -> T,
) -> T {
    if little_endian { little(bytes) } else { big(bytes) }
}
