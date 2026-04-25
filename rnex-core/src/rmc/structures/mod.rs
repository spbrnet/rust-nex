use crate::rmc::structures::helpers::DummyWriter;
use std::io::{Read, Write};
use std::string::FromUtf8Error;
use std::{fmt, io};
use thiserror::Error;
//ideas for the future: make a proc macro library which allows generation of struct reads

#[derive(Error, Debug)]
pub enum Error {
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
    FormatError(#[from] fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod any;
pub mod buffer;
pub mod connection_data;
pub mod data;
pub mod helpers;
pub mod list;
pub mod matchmake;
pub mod networking;
pub mod primitives;
pub mod qbuffer;
pub mod qresult;
pub mod ranking;
pub mod rmc_struct;
pub mod string;
pub mod variant;

pub trait RmcSerialize {
    fn serialize(&self, writer: &mut impl Write) -> Result<()>;
    fn serialize_write_size(&self) -> Result<u32> {
        let mut dummy = DummyWriter::new();

        self.serialize(&mut dummy)?;

        Ok(dummy.get_total_len())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self>
    where
        Self: Sized;

    fn to_data(&self) -> Result<Vec<u8>> {
        let expected_size = self.serialize_write_size()?;
        let mut data = Vec::with_capacity(expected_size as usize);

        self.serialize(&mut data)?;

        debug_assert_eq!(expected_size, data.len() as u32);

        Ok(data)
    }
    fn name() -> &'static str {
        "NoNameSpecified"
    }
}

impl RmcSerialize for () {
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
