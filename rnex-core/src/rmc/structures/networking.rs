use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4};
use rnex_core::prudp::virtual_port::VirtualPort;
use crate::rmc::structures::RmcSerialize;

impl RmcSerialize for SocketAddrV4{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.ip().to_bits().serialize(writer)?;
        self.port().serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let ip = u32::deserialize(reader)?;
        let port = u16::deserialize(reader)?;

        Ok(SocketAddrV4::new(Ipv4Addr::from_bits(ip), port))
    }
}


impl RmcSerialize for VirtualPort{
    fn serialize(&self, writer: &mut dyn Write) -> crate::rmc::structures::Result<()> {
        self.0.serialize(writer)?;

        Ok(())
    }

    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        Ok(Self(u8::deserialize(reader)?))
    }
}