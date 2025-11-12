use std::{fmt, io};
use std::io::{Read, Write};
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::rmc::structures::helpers::DummyWriter;
//ideas for the future: make a proc macro library which allows generation of struct reads

#[derive(Error, Debug)]
pub enum Error{
    #[error("Io Error: {0}")]
    Io(#[from] io::Error),
    #[error("UTF8 conversion Error: {0}")]
    Utf8(#[from] FromUtf8Error),
    #[error("unexpected value: {0}")]
    UnexpectedValue(u64),
    #[error("version mismatch: {0}")]
    VersionMismatch(u8),
    #[error("an error occurred reading the station url")]
    StationUrlInvalid,
    #[error("error formatting text: {0}")]
    FormatError(#[from] fmt::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod string;
pub mod any;
pub mod qresult;
pub mod buffer;
pub mod connection_data;
pub mod rmc_struct;
pub mod list;
pub mod qbuffer;
pub mod primitives;
pub mod matchmake;
pub mod variant;
pub mod ranking;
pub mod networking;
pub mod helpers;

pub trait RmcSerialize{
    fn serialize(&self, writer: &mut impl Write) -> Result<()>;
    fn serialize_write_size(&self) -> Result<u32>{
        let mut dummy = DummyWriter::new();

        self.serialize(&mut dummy)?;

        Ok(dummy.get_total_len())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> where Self: Sized;

    fn to_data(&self) -> Result<Vec<u8>>{
        let mut data = Vec::with_capacity(
            self.serialize_write_size()? as usize
        );

        self.serialize(&mut data)?;

        debug_assert_eq!(self.serialize_write_size().unwrap(), data.len() as u32);

        Ok(data)
    }
}

impl RmcSerialize for (){
    fn serialize(&self, _writer: &mut impl Write) -> Result<()> {
        Ok(())
    }
    fn deserialize(_reader: &mut impl Read) -> Result<Self> {
        Ok(())
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(0)
    }

    
}