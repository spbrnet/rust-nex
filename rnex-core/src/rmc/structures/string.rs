use std::io::{Read, Write};
use bytemuck::bytes_of;
use log::error;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use super::{Result, RmcSerialize};

impl RmcSerialize for String{
    fn deserialize(mut reader: &mut dyn Read) -> Result<Self> {
        let len: u16 = reader.read_struct(IS_BIG_ENDIAN)?;
        let mut data = vec![0; len as usize - 1];
        reader.read_exact(&mut data)?;

        let null: u8 = reader.read_struct(IS_BIG_ENDIAN)?;
        if null != 0{
            error!("unable to find null terminator... continuing anyways");
        }

        Ok(String::from_utf8(data)?)
    }
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        (&self[..]).serialize(writer)
    }
}

impl RmcSerialize for &str{
    fn deserialize(_reader: &mut dyn Read) -> Result<Self> {
        panic!("cannot serialize to &str")
    }
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        let u16_len: u16 = (self.len() + 1) as u16;
        writer.write(bytes_of(&u16_len))?;

        writer.write(self.as_bytes())?;
        writer.write(&[0])?;

        Ok(())
    }
}

