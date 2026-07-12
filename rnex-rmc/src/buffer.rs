use std::io::{Read, Write};

use crate::serialization::{Result, RmcSerialize};

impl<'a> RmcSerialize for &'a [u8] {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        let u32_size = self.len() as u32;
        writer.write(bytemuck::bytes_of(&u32_size))?;
        writer.write(self)?;

        Ok(())
    }

    /// DO NOT USE (also maybe split off the serialize and deserialize functions at some point)
    fn deserialize(_reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        panic!("cannot deserialize to a u8 slice reference (use this ONLY for writing)")
    }

    fn serialize_write_size(&self) -> Result<u32> {
        Ok(4 + self.len() as u32)
    }
}

impl RmcSerialize for Box<[u8]> {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        Vec::deserialize(reader).map(|v| v.into_boxed_slice())
    }

    fn serialize_write_size(&self) -> Result<u32> {
        (&self[..]).serialize_write_size()
    }
}
