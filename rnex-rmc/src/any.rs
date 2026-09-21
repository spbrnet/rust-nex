use crate::{
    RmcSerialize,
    serialization::{Error, Result, RmcStruct},
};
use std::{
    io::{Cursor, Read, Write},
    marker::PhantomData,
};
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};

use crate::data::Data;

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
            return Err(Error::InheritanceError);
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
            return Err(Error::InheritanceError);
        }
        Ok(Any {
            data: self.data,
            name: self.name,
            phantom_data: PhantomData,
        })
    }
    pub fn try_get_as<U: RmcStruct>(&self) -> Result<U> {
        if !U::get_struct_info().is_inheritor(&self.name) {
            return Err(Error::InheritanceError);
        }
        return U::deserialize(&mut Cursor::new(&self.data[..]));
    }
    pub fn new<U: RmcStruct>(val: &U) -> Result<Self> {
        if !T::get_struct_info().is_inheritor(U::get_struct_info().name) {
            return Err(Error::InheritanceError);
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
            return Err(Error::InheritanceError);
        }

        let mut cur = Cursor::new(&self.data[..]);
        // skip the parent part of the struct
        let _ = T::deserialize(&mut cur)?;
        let end_of_parent_pos = cur.position();
        let rest_of_struct = &self
            .data
            .get(end_of_parent_pos as usize..)
            .ok_or(Error::OOB)?;

        let mut new_data = parent.to_data()?;
        new_data.extend_from_slice(&rest_of_struct);

        self.data = new_data;

        Ok(())
    }
}
