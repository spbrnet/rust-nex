use crate::rmc::response::ErrorCode;
use crate::rmc::structures::{Result, RmcSerialize};
use bytemuck::{Pod, Zeroable, bytes_of};
use std::io::{Read, Write};
use v_byte_helpers::SwapEndian;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

pub const ERROR_MASK: u32 = 1 << 31;

#[derive(Pod, Zeroable, Copy, Clone, SwapEndian, Debug)]
#[repr(transparent)]
pub struct QResult(u32);

impl QResult {
    pub fn success(error_code: ErrorCode) -> Self {
        let val: u32 = error_code.into();

        Self(val & (!ERROR_MASK))
    }

    pub fn error(error_code: ErrorCode) -> Self {
        let val: u32 = error_code.into();

        Self(val | ERROR_MASK)
    }
}

impl RmcSerialize for QResult {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        writer.write(bytes_of(self))?;
        Ok(())
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let result: Self = reader.read_struct(IS_BIG_ENDIAN)?;
        println!("reading qresult: {:x}", result.0);
        Ok(result)
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(4)
    }
}
