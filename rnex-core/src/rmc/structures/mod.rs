use crate::rmc::structures::helpers::DummyWriter;
use async_trait::async_trait;
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
    #[cfg(feature = "rmc_struct_header")]
    #[error("version mismatch: {0}")]
    VersionMismatch(u8),
    #[error("an error occurred reading the station url")]
    StationUrlInvalid,
    #[error("error formatting text: {0}")]
    FormatError(#[from] fmt::Error),
    #[error("uncategorized rmc error occurred: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
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
pub mod resultsrange;
pub mod rmc_struct;
pub mod string;
pub mod string_set;
pub mod variant;

pub trait RmcSerialize {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()>;
    fn serialize_write_size(&self) -> Result<u32> {
        let mut dummy = DummyWriter::new();

        self.serialize(&mut dummy)?;

        Ok(dummy.get_total_len())
    }
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self>
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
    fn version() -> Option<u8> {
        None
    }
}

trait SendWrite: Send + Write {}
impl<T: Send + Write> SendWrite for T {}

// beware that this trait throws away most of the optimizations which come with using
// the regular `Serialize` trait, ONLY use this when it is 100% required to have dyn
// compatibility
#[async_trait]
trait DynRmcSerialize: Send + Sync {
    async fn serialize(&self, writer: &mut dyn SendWrite) -> Result<()>;
}

#[async_trait]
impl<T: RmcSerialize + Send + Sync> DynRmcSerialize for T {
    async fn serialize(&self, writer: &mut dyn SendWrite) -> Result<()> {
        <Self as RmcSerialize>::serialize(&self, writer)
    }
}

impl RmcSerialize for () {
    fn serialize(&self, _writer: &mut (impl Write + ?Sized)) -> Result<()> {
        Ok(())
    }
    fn deserialize(_reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        Ok(())
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(0)
    }
}

trait RmcInternalAnyUnknownAs<T: RmcStruct>: AsRef<T> + DynRmcSerialize + RmcStructInstance {}

trait RmcCastable {
    // consumes box and returns a Box containing a Box with the requested Struct details
    fn cast_to(self: Box<Self>, destination: &RmcStructInfo) -> Box<dyn std::any::Any>;
}

struct RmcStructInfo {
    inheritors: Vec<&'static RmcStructInfo>,
    name: &'static str,
    deserialize_abstract: fn(&mut dyn Read) -> Box<dyn std::any::Any>,
}

impl RmcStructInfo {
    fn deserialze_abstract_as<T: RmcStruct>(
        reader: &impl Read,
    ) -> Box<dyn RmcInternalAnyUnknownAs<T>> {
        todo!()
    }
}

trait RmcStruct {
    fn get_struct_info() -> &'static RmcStructInfo;
}

trait RmcStructInstance {
    fn get_self_struct_info(&self) -> &'static RmcStructInfo;
}
