use std::io::{Read, Write};
use crate::rmc::structures::RmcSerialize;



impl<'a> RmcSerialize for &'a [u8]{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        let u32_size = self.len() as u32;
        writer.write(bytemuck::bytes_of(&u32_size))?;
        writer.write(self)?;

        Ok(())
    }

    /// DO NOT USE (also maybe split off the serialize and deserialize functions at some point)
    fn deserialize(_reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        panic!("cannot deserialize to a u8 slice reference (use this ONLY for writing)")
    }
}

impl RmcSerialize for Box<[u8]>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        (&self[..]).serialize(writer)
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Vec::deserialize(reader).map(|v| v.into_boxed_slice())
    }
}