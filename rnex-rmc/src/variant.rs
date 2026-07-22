use std::io::{Read, Write};

use rnex_util::date_time::DateTime;

use crate::serialization::{Error, Result, RmcSerialize};

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Variant {
    #[default]
    None,
    SInt64(i64),
    Double(f64),
    Bool(bool),
    String(String),
    DateTime(DateTime),
    UInt64(u64),
}

impl RmcSerialize for Variant {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        match self {
            Variant::None => {
                writer.write_all(&[0])?;
            }
            Variant::SInt64(v) => {
                writer.write_all(&[1])?;
                v.serialize(writer)?;
            }
            Variant::Double(v) => {
                writer.write_all(&[2])?;
                v.serialize(writer)?;
            }
            Variant::Bool(v) => {
                writer.write_all(&[3])?;
                v.serialize(writer)?;
            }
            Variant::String(v) => {
                writer.write_all(&[4])?;
                v.serialize(writer)?;
            }
            Variant::DateTime(v) => {
                writer.write_all(&[5])?;
                v.serialize(writer)?;
            }
            Variant::UInt64(v) => {
                writer.write_all(&[6])?;
                v.serialize(writer)?;
            }
        }

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        match u8::deserialize(reader)? {
            0 => Ok(Variant::None),
            1 => Ok(Variant::SInt64(i64::deserialize(reader)?)),
            2 => Ok(Variant::Double(f64::deserialize(reader)?)),
            3 => Ok(Variant::Bool(bool::deserialize(reader)?)),
            4 => Ok(Variant::String(String::deserialize(reader)?)),
            5 => Ok(Variant::DateTime(DateTime::deserialize(reader)?)),
            6 => Ok(Variant::UInt64(u64::deserialize(reader)?)),
            v => Err(Error::UnexpectedValue(v as u64)),
        }
    }
}
