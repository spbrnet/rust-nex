use bytemuck::bytes_of;
use std::io::{Read, Write};
use tracing::error;
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};

use crate::serialization::{Result, RmcSerialize};

impl RmcSerialize for String {
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let len: u16 = reader.read_struct(IS_BIG_ENDIAN)?;
        if len == 0 {
            return Ok("".to_string());
        }
        let mut data = vec![0; len as usize];
        reader.read_exact(&mut data)?;
        if *data.last().unwrap() != 0 {
            error!("unable to find null terminator... continuing anyways");
        }
        data.pop();

        Ok(String::from_utf8(data)?)
    }
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        (&self[..]).serialize(writer)
    }
    fn serialize_write_size(&self) -> Result<u32> {
        (&self[..]).serialize_write_size()
    }
}

impl RmcSerialize for &str {
    fn deserialize(_reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        panic!("cannot serialize to &str")
    }
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        let u16_len: u16 = (self.len() + 1) as u16;
        writer.write_all(bytes_of(&u16_len))?;

        writer.write_all(self.as_bytes())?;
        writer.write_all(&[0])?;

        Ok(())
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(2 + self.as_bytes().len() as u32 + 1)
    }
}
