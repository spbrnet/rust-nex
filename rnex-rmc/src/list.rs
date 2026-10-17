use bytemuck::bytes_of;
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};
use std::io::{Read, Write};

use crate::serialization::{Result, RmcSerialize};

pub type Buffer = Vec<u8>;
pub type List<T> = Vec<T>;

// this is also for implementing `Buffer` this is tecnically not the same as its handled internaly
// probably but as it has the same mapping it doesn't matter and simplifies things
impl<T: RmcSerialize> RmcSerialize for Vec<T> {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        let u32_len = self.len() as u32;

        writer.write_all(bytes_of(&u32_len))?;
        for e in self {
            e.serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let len: u32 = reader.read_struct(IS_BIG_ENDIAN)?;
        if len as usize > rnex_util::MAX_PACKET_SIZE / std::mem::size_of::<T>().max(1) {
            return Err(crate::serialization::Error::OOB);
        }
        let mut vec = Vec::new();
        for _ in 0..len {
            vec.push(T::deserialize(reader)?);
        }

        Ok(vec)
    }

    fn serialize_write_size(&self) -> Result<u32> {
        let mut val = 0u32;
        for i in self {
            val += i.serialize_write_size()?;
        }
        Ok(4 + val)
    }
}

impl<const LEN: usize, T: RmcSerialize> RmcSerialize for [T; LEN] {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        for i in 0..LEN {
            self[i].serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let mut values = Vec::with_capacity(LEN);
        for _ in 0..LEN {
            values.push(T::deserialize(reader)?);
        }
        values
            .try_into()
            .map_err(|_| crate::serialization::Error::OOB)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> Result<u32> {
        let mut val = 0u32;
        for i in self {
            val += i.serialize_write_size()?;
        }
        Ok(val)
    }
}
