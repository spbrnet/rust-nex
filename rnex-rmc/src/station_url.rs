use std::fmt::Write;
use std::io::{self, Read};

use rnex_util::station_url::StationUrl;

use crate::{
    helpers::DummyFormatWriter,
    serialization::{Error::StationUrlInvalid, Result, RmcSerialize},
};

impl RmcSerialize for StationUrl {
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let str = String::deserialize(reader)?;

        Self::try_from(str.as_str()).map_err(|_| StationUrlInvalid)
    }
    fn serialize(&self, writer: &mut (impl io::Write + ?Sized)) -> Result<()> {
        let str: String = self.into();

        str.serialize(writer)
    }

    fn serialize_write_size(&self) -> Result<u32> {
        let mut dummy = DummyFormatWriter::new();

        write!(&mut dummy, "{}", self)?;

        Ok(dummy.serialize_str_len())
    }
}
