use rnex_core::rmc::structures::{Result, RmcSerialize};
use std::{
    io::{Cursor, Read, Write},
    marker::PhantomData,
};
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

use crate::rmc::structures::{RmcStruct, data::Data};

#[derive(Clone, Debug)]
pub struct Any<T: RmcStruct = Data> {
    pub name: String,
    pub data: Vec<u8>,
    pub phantom_data: PhantomData<T>,
}

impl<T: RmcStruct> RmcSerialize for Any<T> {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.name.serialize(writer)?;

        let u32_len = self.data.len() as u32;
        (u32_len + 4).serialize(writer)?;
        self.data.serialize(writer)?;

        Ok(())
    }
    fn deserialize(mut reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let name = String::deserialize(reader)?;

        if !T::get_struct_info().is_inheritor(&name) {
            return Err(super::Error::InheritanceError);
        }

        // also length ?
        let _len2: u32 = reader.read_struct(IS_BIG_ENDIAN)?;
        let data = Vec::deserialize(reader)?;

        Ok(Any {
            name,
            data,
            phantom_data: PhantomData,
        })
    }
}

impl<T: RmcStruct> Any<T> {
    pub fn try_into<U: RmcStruct>(self) -> Result<Any<U>> {
        if !U::get_struct_info().is_inheritor(&self.name) {
            return Err(super::Error::InheritanceError);
        }
        Ok(Any {
            data: self.data,
            name: self.name,
            phantom_data: PhantomData,
        })
    }
    pub fn try_get_as<U: RmcStruct>(&self) -> Result<U> {
        if !U::get_struct_info().is_inheritor(&self.name) {
            return Err(super::Error::InheritanceError);
        }
        return U::deserialize(&mut Cursor::new(&self.data[..]));
    }
    pub fn new<U: RmcStruct>(val: &U) -> Result<Self> {
        if !T::get_struct_info().is_inheritor(U::get_struct_info().name) {
            return Err(super::Error::InheritanceError);
        }
        return Ok(Self {
            name: U::get_struct_info().name.to_owned(),
            data: val.to_data()?,
            phantom_data: PhantomData,
        });
    }

    pub fn get(&self) -> Result<T> {
        return T::deserialize(&mut Cursor::new(&self.data[..]));
    }

    pub fn emplace_parent<E: RmcStruct>(&mut self, parent: &E) -> Result<()> {
        // validate if E is actually a parent of the contained struct
        if !E::get_struct_info().is_inheritor(&self.name) {
            return Err(super::Error::InheritanceError);
        }

        let mut cur = Cursor::new(&self.data[..]);
        // skip the parent part of the struct
        let _ = T::deserialize(&mut cur)?;
        let end_of_parent_pos = cur.position();
        let rest_of_struct = &self
            .data
            .get(end_of_parent_pos as usize..)
            .ok_or(super::Error::OOB)?;

        let mut new_data = parent.to_data()?;
        new_data.extend_from_slice(&rest_of_struct);

        self.data = new_data;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use macros::RmcSerialize;

    use crate::rmc::structures::RmcSerialize;
    use crate::rmc::structures::any::Any;
    #[derive(RmcSerialize)]
    #[rmc_struct(0)]
    struct StructB {
        valb: u32,
    }
    #[derive(RmcSerialize)]
    #[rmc_struct(0)]
    struct StructA {
        #[extends]
        base: StructB,
        vala: u32,
    }
    #[derive(RmcSerialize)]
    #[rmc_struct(0)]
    struct Unrelated {
        vala: u32,
    }

    #[test]
    fn test() {
        let initial: Any<StructA> = Any::new(&StructA {
            base: StructB { valb: 10 },
            vala: 11,
        })
        .unwrap();

        let encoded = initial.to_data().unwrap();

        let mut reser_b: Any<StructB> = Any::deserialize(&mut Cursor::new(&encoded[..])).unwrap();

        let mut data_b: StructB = reser_b.get().unwrap();
        assert_eq!(data_b.valb, 10);
        data_b.valb = 20;

        reser_b.emplace_parent(&data_b).unwrap();

        let data_a: StructA = reser_b.try_get_as().unwrap();
        assert_eq!(data_a.vala, 11);
        assert_eq!(data_a.base.valb, 20);

        let data: Any<StructA> = reser_b.try_into().unwrap();
        let data: Any<StructB> = data.try_into().unwrap();
        let fail: Result<Any<Unrelated>, _> = data.try_into();
        assert!(fail.is_err());
        let fail: Result<Any<Unrelated>, _> = Any::deserialize(&mut Cursor::new(&encoded[..]));
        assert!(fail.is_err());
    }
}
