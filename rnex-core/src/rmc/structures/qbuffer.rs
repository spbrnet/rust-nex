use std::io::{Read, Write};
use bytemuck::bytes_of;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::{Result, RmcSerialize};


#[derive(Debug)]
pub struct QBuffer(pub Vec<u8>);

impl RmcSerialize for QBuffer{
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        let len_u16 = self.0.len() as u16;

        writer.write(bytes_of(&len_u16))?;
        writer.write(&self.0)?;

        Ok(())
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let size: u16 = reader.read_struct(IS_BIG_ENDIAN)?;

        let mut vec = vec![0; size as usize];

        reader.read_exact(&mut vec)?;

        Ok(Self(vec))
    }
}