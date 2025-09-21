use std::io::{Read, Write};
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use super::{Result, RmcSerialize};

#[derive(Debug, Default)]
pub struct Any{
    pub name: String,
    pub data: Vec<u8>
}

impl RmcSerialize for Any{
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        self.name.serialize(writer)?;

        let u32_len = self.data.len() as u32;

        u32_len.serialize(writer)?;
        u32_len.serialize(writer)?;

        self.data.serialize(writer)?;

        Ok(())
    }
    fn deserialize(mut reader: &mut dyn Read) -> Result<Self> {
        let name = String::deserialize(reader)?;

        // also length ?
        let _len2: u32 = reader.read_struct(IS_BIG_ENDIAN)?;
        let length: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

        let mut data = vec![0; length as usize];

        reader.read_exact(&mut data)?;

        Ok(
            Any{
                name,
                data
            }
        )
    }
}