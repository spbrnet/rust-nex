use std::io::{Read, Write};
use bytemuck::bytes_of;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::RmcSerialize;

impl RmcSerialize for u8{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for i8{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u16{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for i16{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u32{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for i32{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for u64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for i64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for f64{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        Ok(writer.write_all(bytes_of(self))?)
    }

    fn deserialize(mut reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(reader.read_struct(IS_BIG_ENDIAN)?)
    }
}

impl RmcSerialize for bool{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        match self{
            true => writer.write_all(&[1])?,
            false => writer.write_all(&[0])?,
        }
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(u8::deserialize(reader)? != 0)
    }
}


impl<T: RmcSerialize, U: RmcSerialize> RmcSerialize for (T, U){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;

        Ok((first, second))
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize> RmcSerialize for (T, U, V){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;

        Ok((first, second, third))
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize> RmcSerialize for (T, U, V, W){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;

        Ok((first, second, third, fourth))
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize> RmcSerialize for (T, U, V, W, X){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth))
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize, Y: RmcSerialize> RmcSerialize for (T, U, V, W, X, Y){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;
        self.5.serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;
        let sixth = Y::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth, sixth))
    }
}

impl<T: RmcSerialize, U: RmcSerialize, V: RmcSerialize, W: RmcSerialize, X: RmcSerialize, Y: RmcSerialize, Z: RmcSerialize> RmcSerialize for (T, U, V, W, X, Y, Z){
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;
        self.1.serialize(writer)?;
        self.2.serialize(writer)?;
        self.3.serialize(writer)?;
        self.4.serialize(writer)?;
        self.5.serialize(writer)?;
        self.6.serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let first = T::deserialize(reader)?;
        let second = U::deserialize(reader)?;
        let third = V::deserialize(reader)?;
        let fourth = W::deserialize(reader)?;
        let fifth = X::deserialize(reader)?;
        let sixth = Y::deserialize(reader)?;
        let seventh = Z::deserialize(reader)?;

        Ok((first, second, third, fourth, fifth, sixth, seventh))
    }
}

impl<T: RmcSerialize> RmcSerialize for Box<T>{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.as_ref().serialize(writer)
    }
    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        T::deserialize(reader).map(Box::new)
    }
}