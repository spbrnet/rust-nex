use std::io;
use std::io::{Read, Write};
use std::string::FromUtf8Error;
use thiserror::Error;

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
    StationUrlInvalid
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
mod networking;

pub trait RmcSerialize{
    fn serialize(&self, writer: &mut dyn Write) -> Result<()>;
    fn deserialize(reader: &mut dyn Read) -> Result<Self> where Self: Sized;

    fn to_data(&self) -> Vec<u8>{
        let mut data = Vec::new();

        self.serialize(&mut data).expect("out of memory or something");

        data
    }
}

impl RmcSerialize for (){
    fn serialize(&self, writer: &mut dyn Write) -> Result<()> {
        Ok(())
    }
    fn deserialize(reader: &mut dyn Read) -> Result<Self> {
        Ok(())
    }

    
}