use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};

use crate::serialization::{Error, Result, RmcSerialize};

impl RmcSerialize for SocketAddr {
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self>
    where
        Self: Sized,
    {
        let val: u8 = reader.read_struct(IS_BIG_ENDIAN)?;
        match val {
            4 => Ok(SocketAddr::V4(SocketAddrV4::deserialize(reader)?)),
            6 => Ok(SocketAddr::V6(SocketAddrV6::deserialize(reader)?)),
            v => Err(Error::UnexpectedValue(v as u64)),
        }
    }
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        match self {
            SocketAddr::V4(v) => {
                writer.write_all(&[4])?;
                v.serialize(writer)?;
            }
            SocketAddr::V6(v) => {
                writer.write_all(&[6])?;
                v.serialize(writer)?;
            }
        }
        Ok(())
    }
}

impl RmcSerialize for SocketAddrV4 {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.ip().to_bits().serialize(writer)?;
        self.port().serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let ip = u32::deserialize(reader)?;
        let port = u16::deserialize(reader)?;

        Ok(SocketAddrV4::new(Ipv4Addr::from_bits(ip), port))
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(6)
    }
}
impl RmcSerialize for SocketAddrV6 {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.ip().to_bits().serialize(writer)?;
        self.port().serialize(writer)?;
        self.flowinfo().serialize(writer)?;
        self.scope_id().serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        let ip = u128::deserialize(reader)?;
        let port = u16::deserialize(reader)?;
        let flowinfo = u32::deserialize(reader)?;
        let scope_id = u32::deserialize(reader)?;

        Ok(SocketAddrV6::new(
            Ipv6Addr::from_bits(ip),
            port,
            flowinfo,
            scope_id,
        ))
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(26)
    }
}
/*
use rnex_core::prudp::virtual_port::VirtualPort;
impl RmcSerialize for VirtualPort {
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.0.serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        Ok(Self(u8::deserialize(reader)?))
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(1)
    }
}*/
