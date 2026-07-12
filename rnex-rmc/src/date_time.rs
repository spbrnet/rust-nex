use std::io::{Read, Write};

use rnex_util::date_time::DateTime;

use crate::serialization::{Result, RmcSerialize};

impl RmcSerialize for DateTime {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.0.serialize(writer)
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        Ok(Self(u64::deserialize(reader)?))
    }
}
