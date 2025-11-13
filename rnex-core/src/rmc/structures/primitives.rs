use std::io::{Read, Write};
use bytemuck::bytes_of;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::RmcSerialize;

impl RmcSerialize for u8{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(1)
    }
}

impl RmcSerialize for i8{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(1)
    }
}

impl RmcSerialize for u16{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(2)
    }
}

impl RmcSerialize for i16{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(2)
    }
}

impl RmcSerialize for u32{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(4)
    }
}

impl RmcSerialize for i32{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(4)
    }
}

impl RmcSerialize for u64{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(8)
    }
}

impl RmcSerialize for i64{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(8)
    }
}

impl RmcSerialize for f64{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(8)
    }
}

impl RmcSerialize for bool{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        match self{
            true => writer.write_all(&[1])?,
            false => writer.write_all(&[0])?,
        }
        Ok(())
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        Ok(u8::deserialize(reader)? != 0)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(1)
    }
}


impl<T: RmcSerialize, U: RmcSerialize> RmcSerialize for (T, U){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        Ok(())
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;

        Ok((first, second))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
            self.1.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize> RmcSerialize for (T, U, V){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        Ok(())
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;

        Ok((first, second, third))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
            self.1.serialize_write_size()? +
            self.2.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize> RmcSerialize for (T, U, V, W){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        Ok(())
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;

        Ok((first, second, third, fourth))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
            self.1.serialize_write_size()? +
            self.2.serialize_write_size()? +
            self.3.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize> RmcSerialize for (T, U, V, W, X){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;

        Ok(())
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
            self.1.serialize_write_size()? +
            self.2.serialize_write_size()? +
            self.2.serialize_write_size()? +
            self.3.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize, Y: RmcSerialize> RmcSerialize for (T, U, V, W, X, Y){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;
        self.5.serialize(writer)?;

        Ok(())
    }

    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;
        let sixth = Y::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth, sixth))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
                self.1.serialize_write_size()? +
                self.2.serialize_write_size()? +
                self.3.serialize_write_size()? +
                self.4.serialize_write_size()? +
                self.5.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize, Y: RmcSerialize, Z: RmcSerialize> RmcSerialize for (T, U, V, W, X, Y, Z){
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;
        self.5.serialize(writer)?;
        self.6.serialize(writer)?;

        Ok(())
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;
        let sixth = Y::deserialize(reader)?;
        let seventh = Z::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth, sixth, seventh))
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        Ok(
            self.0.serialize_write_size()? +
                self.1.serialize_write_size()? +
                self.2.serialize_write_size()? +
                self.3.serialize_write_size()? +
                self.4.serialize_write_size()? +
                self.5.serialize_write_size()? +
                self.6.serialize_write_size()?
        )
    }
}

impl<T: RmcSerialize> RmcSerialize for Box<T>{
    #[inline(always)]
    fn serialize(&self, writer: &mut impl Write) -> crate::rmc::structures::Result<()> {
        self.as_ref().serialize(writer)
    }
    #[inline(always)]
    fn deserialize(reader: &mut impl Read) -> crate::rmc::structures::Result<Self> {
        T::deserialize(reader).map(Box::new)
    }
    #[inline(always)]
    fn serialize_write_size(&self) -> crate::rmc::structures::Result<u32> {
        T::serialize_write_size(self.as_ref())
    }
}