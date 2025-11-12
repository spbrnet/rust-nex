use std::io::{Read, Write};
use bytemuck::{bytes_of, Pod, Zeroable};
use v_byte_helpers::SwapEndian;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::{RmcSerialize, Result};

pub const ERROR_MASK: u32 =  1 << 31;

#[derive(Pod, Zeroable, Copy, Clone, SwapEndian, Debug)]
#[repr(transparent)]
pub struct QResult(u32);

impl QResult{
    pub fn success(error_code: ErrorCode) -> Self{
        let val: u32 = error_code.into();

        Self(val & (!ERROR_MASK))
    }

    pub fn error(error_code: ErrorCode) -> Self{
        let val: u32 = error_code.into();

        Self(val | ERROR_MASK)
    }
}

impl RmcSerialize for QResult{
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        writer.write(bytes_of(self))?;
        Ok(())
    }

    fn deserialize(mut reader: &mut impl Read) -> Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}