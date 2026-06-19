use crate::rmc::structures::RmcSerialize;
use bytemuck::bytes_of;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

// this is also for implementing `Buffer` this is tecnically not the same as its handled internaly
// probably but as it has the same mapping it doesn't matter and simplifies things
impl<T: RmcSerialize> RmcSerialize for Vec<T> {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> crate::rmc::structures::Result<()> {
        let u32_len = self.len() as u32;

        writer.write_all(bytes_of(&u32_len))?;
        for e in self {
            e.serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(mut reader: &mut (impl Read + ?Sized)) -> crate::rmc::structures::Result<Self> {
        println!("reading list");
        let len: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

        println!("readijg list: {:?}", len);
        //let mut vec = Vec::with_capacity(len as usize);

        let vec: Vec<T> = (0..len)
            .map(|_| T::deserialize(reader))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(vec)
    }

    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        let mut val = 0u32;
        for i in self {
            val += i.serialize_write_size()?;
        }
        Ok(4 + val)
    }
}

impl<const LEN: usize, T: RmcSerialize> RmcSerialize for [T; LEN] {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> crate::rmc::structures::Result<()> {
        for i in 0..LEN {
            self[i].serialize(writer)?;
        }

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> crate::rmc::structures::Result<Self> {
        let mut arr = [const { MaybeUninit::<T>::uninit() }; LEN];

        for i in 0..LEN {
            arr[i] = MaybeUninit::new(T::deserialize(reader)?);
        }

        // all of the elements are now initialized so it is safe to assume they are initialized

        let arr = arr.map(|v| unsafe { v.assume_init() });

        Ok(arr)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        let mut val = 0u32;
        for i in self {
            val += i.serialize_write_size()?;
        }
        Ok(val)
    }
}
