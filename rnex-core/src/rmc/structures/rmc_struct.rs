use std::io::{Cursor, Read, Write};
use bytemuck::bytes_of;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::structures::Error::VersionMismatch;
use crate::rmc::structures::Result;

#[repr(C, packed)]
struct StructureHeader{
    version: u8,
    length: u32
}

pub fn write_struct(writer: &mut dyn Write, version: u8, pred: impl FnOnce(&mut Vec<u8>) -> Result<()> ) -> Result<()> {
    writer.write_all(&[version])?;

    let mut scratch_space: Vec<u8> = Vec::new();

    (pred)(&mut scratch_space)?;

    let u32_size = scratch_space.len() as u32;

    writer.write_all(bytes_of(&u32_size))?;
    writer.write_all(&scratch_space)?;

    Ok(())
}

pub fn read_struct<T: Sized>(mut reader: &mut dyn Read, version: u8, pred: impl FnOnce(&mut Cursor<Vec<u8>>) -> Result<T>) -> Result<T> {
    let ver: u8 = reader.read_struct(IS_BIG_ENDIAN)?;

    if ver != version{
        return Err(VersionMismatch(ver));
    }

    let size: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

    let mut vec = vec![0u8; size as usize];

    reader.read_exact(&mut vec)?;

    let mut cursor = Cursor::new(vec);

    Ok(pred(&mut cursor)?)
}