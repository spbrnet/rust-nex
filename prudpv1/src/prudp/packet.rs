// no clue why this produces a warning where `#[repr(u16)]` is below,
// the thing is says to do also breaks the code, so we just
// force the compiler to shut up here
#![allow(unused_parens)]

use crate::prudp::packet::PacketOption::{
    ConnectionSignature, FragmentId, InitialSequenceId, MaximumSubstreamId, SupportedFunctions,
};
use bytemuck::{Pod, Zeroable};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rc4::KeyInit;
use rnex_prudp::socket_addr::PRUDPSockAddr;
use rnex_prudp::types_flags::TypesFlags;
use rnex_prudp::types_flags::flags::ACK;
use rnex_prudp::virtual_port::VirtualPort;
use std::fmt::Debug;
use std::io;
use std::io::{Cursor, Read, Seek, Write};
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use thiserror::Error;
use tracing::{error, warn};
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions, SwapEndian};

type Md5Hmac = Hmac<Md5>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("invalid magic {0:#06x}")]
    InvalidMagic(u16),
    #[error("invalid version {0}")]
    InvalidVersion(u8),
    #[error("invalid option id {0}")]
    InvalidOptionId(u8),
    #[error("option size {size} doesnt match expected option for given option id {id}")]
    InvalidOptionSize { id: u8, size: u8 },
}

pub type Result<T> = std::result::Result<T, Error>;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable, Eq, PartialEq)]
pub struct PRUDPV1Header {
    pub magic: [u8; 2],
    pub version: u8,
    pub packet_specific_size: u8,
    pub payload_size: u16,
    pub source_port: VirtualPort,
    pub destination_port: VirtualPort,
    pub types_and_flags: TypesFlags,
    pub session_id: u8,
    pub substream_id: u8,
    pub sequence_id: u16,
}

impl SwapEndian for PRUDPV1Header {
    fn swap_endian(self) -> Self {
        Self {
            magic: self.magic,
            version: self.version,
            packet_specific_size: self.packet_specific_size,
            payload_size: self.payload_size.swap_bytes(),
            source_port: self.source_port.swap_endian(),
            destination_port: self.destination_port.swap_endian(),
            types_and_flags: self.types_and_flags.swap_endian(),
            session_id: self.session_id,
            substream_id: self.substream_id,
            sequence_id: self.sequence_id.swap_bytes(),
        }
    }
}

impl Default for PRUDPV1Header {
    fn default() -> Self {
        Self {
            magic: [0xEA, 0xD0],
            version: 1,
            session_id: 0,
            source_port: VirtualPort(0),
            sequence_id: 0,
            payload_size: 0,
            destination_port: VirtualPort(0),
            types_and_flags: TypesFlags(0),
            packet_specific_size: 0,
            substream_id: 0,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PacketOption {
    SupportedFunctions(u32),
    ConnectionSignature([u8; 16]),
    FragmentId(u8),
    InitialSequenceId(u16),
    MaximumSubstreamId(u8),
}

impl PacketOption {
    fn from(option_id: OptionId, option_data: &[u8]) -> io::Result<Self> {
        let mut data_cursor = Cursor::new(option_data);
        let val = match option_id.into() {
            0 => SupportedFunctions(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            1 => ConnectionSignature(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            2 => FragmentId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            3 => InitialSequenceId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            4 => MaximumSubstreamId(data_cursor.read_struct(IS_BIG_ENDIAN)?),
            _ => unreachable!(),
        };

        Ok(val)
    }

    fn write_to_stream(&self, stream: &mut impl Write) -> io::Result<()> {
        match self {
            SupportedFunctions(v) => {
                stream.write_all(&[0, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            ConnectionSignature(v) => {
                stream.write_all(&[1, size_of_val(v) as u8])?;
                stream.write_all(v)?;
            }
            FragmentId(v) => {
                stream.write_all(&[2, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            InitialSequenceId(v) => {
                stream.write_all(&[3, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
            MaximumSubstreamId(v) => {
                stream.write_all(&[4, size_of_val(v) as u8])?;
                stream.write_all(&v.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn write_size(&self) -> u8 {
        match self {
            SupportedFunctions(_) => 2 + 4,
            ConnectionSignature(_) => 2 + 16,
            FragmentId(_) => 2 + 1,
            InitialSequenceId(_) => 2 + 2,
            MaximumSubstreamId(_) => 2 + 1,
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct PRUDPV1Packet {
    pub header: PRUDPV1Header,
    pub packet_signature: [u8; 16],
    pub payload: Vec<u8>,
    pub options: Vec<PacketOption>,
}

#[derive(Copy, Clone, Debug)]
// Invariant: can only contain 0, 1, 2, 3 or 4
struct OptionId(u8);

impl OptionId {
    fn new(val: u8) -> Result<Self> {
        // Invariant is upheld because we only create the object if it doesn't violate the invariant
        match val {
            0 | 1 | 2 | 3 | 4 => Ok(Self(val)),
            _ => Err(Error::InvalidOptionId(val)),
        }
    }

    fn option_type_size(self) -> u8 {
        match self.0 {
            0 => 4,
            1 => 16,
            2 => 1,
            3 => 2,
            4 => 1,
            _ => unreachable!(),
        }
    }
}

impl Into<u8> for OptionId {
    fn into(self) -> u8 {
        self.0
    }
}

impl PRUDPV1Packet {
    pub fn new(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let header: PRUDPV1Header = reader.read_struct(IS_BIG_ENDIAN)?;

        if header.magic[0] != 0xEA || header.magic[1] != 0xD0 {
            return Err(Error::InvalidMagic(u16::from_be_bytes(header.magic)));
        }

        if header.version != 1 {
            return Err(Error::InvalidVersion(header.version));
        }

        let packet_signature: [u8; 16] = reader.read_struct(IS_BIG_ENDIAN)?;
        //let packet_signature: [u8; 16] = [0; 16];

        assert_eq!(reader.stream_position().ok(), Some(14 + 16));

        let mut packet_specific_buffer = vec![0u8; header.packet_specific_size as usize];

        reader.read_exact(&mut packet_specific_buffer)?;

        //no clue whats up with options but they are broken
        let mut packet_specific_data_cursor = Cursor::new(&packet_specific_buffer);

        let mut options = Vec::new();

        loop {
            let Ok(option_id): io::Result<u8> =
                packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN)
            else {
                break;
            };

            let Ok(value_size): io::Result<u8> =
                packet_specific_data_cursor.read_struct(IS_BIG_ENDIAN)
            else {
                break;
            };

            if value_size == 0 {
                // skip it if its 0 and dont check?
                warn!("reading packets options might be going wrong");
                continue;
            }

            let option_id: OptionId = OptionId::new(option_id)?;

            if option_id.option_type_size() != value_size {
                error!("invalid packet options");
                return Err(Error::InvalidOptionSize {
                    size: value_size,
                    id: option_id.0,
                });
            }

            let mut option_data = vec![0u8; value_size as usize];
            if packet_specific_data_cursor
                .read_exact(&mut option_data[..])
                .is_err()
            {
                error!("unable to read options");
                break;
            }

            options.push(PacketOption::from(option_id, &option_data)?);
        }

        let mut payload = vec![0u8; header.payload_size as usize];

        reader.read_exact(&mut payload)?;

        Ok(Self {
            header,
            packet_signature,
            payload,
            options,
        })
    }

    pub fn base_acknowledgement_packet(&self) -> Self {
        let base = self.base_response_packet();

        let mut flags = self.header.types_and_flags.flags(0);

        flags.set_flag(ACK);

        let options = self
            .options
            .iter()
            .filter(|o| matches!(o, FragmentId(_)))
            .cloned()
            .collect();

        Self {
            header: PRUDPV1Header {
                types_and_flags: flags,
                sequence_id: self.header.sequence_id,
                substream_id: self.header.substream_id,
                session_id: self.header.session_id,
                ..base.header
            },
            options,
            ..base
        }
    }

    pub fn source_sockaddr(&self, socket_addr_v4: SocketAddrV4) -> PRUDPSockAddr {
        PRUDPSockAddr {
            regular_socket_addr: SocketAddr::V4(socket_addr_v4),
            virtual_port: self.header.source_port,
        }
    }

    fn generate_options_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        for option in &self.options {
            option
                .write_to_stream(&mut vec)
                .expect("vec should always automatically be able to extend");
        }

        vec
    }

    pub fn calculate_signature_value(
        &self,
        access_key: &str,
        session_key: Option<[u8; 32]>,
        connection_signature: Option<[u8; 16]>,
    ) -> [u8; 16] {
        let access_key_bytes = access_key.as_bytes();
        let access_key_sum: u32 = access_key_bytes.iter().map(|v| *v as u32).sum();
        let access_key_sum_bytes: [u8; 4] = access_key_sum.to_le_bytes();

        let header_data: [u8; 8] = bytemuck::bytes_of(&self.header)[0x6..].try_into().unwrap();

        let option_bytes = self.generate_options_bytes();

        let mut md5 = md5::Md5::default();

        md5.update(access_key_bytes);
        let key = md5.finalize();

        let mut hmac = Md5Hmac::new_from_slice(&key).expect("fuck");

        hmac.update(&header_data);
        if let Some(session_key) = session_key {
            hmac.update(&session_key);
        }
        hmac.update(&access_key_sum_bytes);
        if let Some(connection_signature) = connection_signature {
            hmac.update(&connection_signature);
        }

        hmac.update(&option_bytes);
        hmac.update(&self.payload);

        hmac.finalize().into_bytes()[0..16]
            .try_into()
            .expect("invalid hmac size")
    }

    pub fn calculate_and_assign_signature(
        &mut self,
        access_key: &str,
        session_key: Option<[u8; 32]>,
        connection_signature: Option<[u8; 16]>,
    ) {
        self.packet_signature =
            self.calculate_signature_value(access_key, session_key, connection_signature);
    }

    pub fn set_sizes(&mut self) {
        self.header.packet_specific_size = self.options.iter().map(|o| o.write_size()).sum();
        self.header.payload_size = self.payload.len() as u16;
    }

    pub fn base_response_packet(&self) -> Self {
        Self {
            header: PRUDPV1Header {
                magic: [0xEA, 0xD0],
                types_and_flags: TypesFlags(0),
                destination_port: self.header.source_port,
                source_port: self.header.destination_port,
                payload_size: 0,
                version: 1,
                packet_specific_size: 0,
                sequence_id: 0,
                session_id: 0,
                substream_id: 0,
            },
            packet_signature: [0; 16],
            payload: Default::default(),
            options: Default::default(),
        }
    }

    pub fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(bytemuck::bytes_of(&self.header))?;
        writer.write_all(&self.packet_signature)?;

        for option in &self.options {
            option.write_to_stream(writer)?;
        }

        writer.write_all(&self.payload)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{OptionId, PRUDPV1Header, PacketOption, TypesFlags};
    use rnex_prudp::{
        types_flags::{
            flags::{NEED_ACK, RELIABLE},
            types::DATA,
        },
        virtual_port::VirtualPort,
    };
    #[test]
    fn size_test() {
        assert_eq!(size_of::<PRUDPV1Header>(), 14);
    }

    #[test]
    fn test_options() {
        let packet_types = [0, 1, 2, 3, 4];

        for p_type in packet_types {
            let option_id = OptionId::new(p_type).unwrap();

            let buf = vec![0; option_id.option_type_size() as usize];

            let opt = PacketOption::from(option_id, &buf).unwrap();
            {
                let mut write_buf = vec![];

                opt.write_to_stream(&mut write_buf).unwrap();

                assert_eq!(write_buf.len() as u8, opt.write_size())
            }
        }
    }

    #[test]
    fn header_read() {
        let header = PRUDPV1Header {
            version: 0,
            destination_port: VirtualPort(0),
            substream_id: 0,
            types_and_flags: TypesFlags(0),
            session_id: 0,
            packet_specific_size: 0,
            payload_size: 0,
            sequence_id: 0,
            magic: [0xEA, 0xD0],
            source_port: VirtualPort(0),
        };

        let bytes = bytemuck::bytes_of(&header);

        let bytes = &bytes[0x6..];

        let _: [u8; 8] = bytes.try_into().unwrap();
    }

    #[test]
    fn test_types_flags() {
        let types = TypesFlags::default().types(DATA).flags(NEED_ACK | RELIABLE);

        assert_ne!((types.0 >> 4) & NEED_ACK, 0);
        assert_ne!((types.0 >> 4) & RELIABLE, 0);
        assert_ne!((types.0 & 0xFF) as u8 & DATA, 0);
    }
}
