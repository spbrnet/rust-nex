use bytemuck::{Pod, Zeroable};
use std::io::{Read, Write};

use crate::{
    response::ErrorCode,
    serialization::{Result, RmcSerialize},
};

pub const ERROR_MASK: u32 = 1 << 31;

#[derive(Pod, Zeroable, Copy, Clone, Debug)]
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
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.0.serialize(writer)
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        RmcSerialize::deserialize(reader).map(Self)
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(4)
    }
}
