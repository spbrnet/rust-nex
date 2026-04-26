use std::{
    fmt::Debug,
    io::{self, Cursor, Read, Write},
};

use bytemuck::{Pod, Zeroable, bytes_of_mut};
use rnex_core::prudp::types_flags::TypesFlags;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

#[derive(Pod, Zeroable, Copy, Clone, Default, Debug)]
#[repr(C)]
pub struct LiteHeader {
    pub magic: u8,
    pub packet_specific_length: u8,
    pub payload_size: u16,
    pub stream_types: StreamTypes,
    pub source_port: u8,
    pub destination_port: u8,
    pub fragment_id: u8,
    pub types_flags: TypesFlags,
    pub sequence_id: u16,
}

pub enum PacketSpecificData {
    SupportedFunctions(u32),
    ConnectionSignature([u8; 16]),
    LiteSignature([u8; 16]),
}

impl PacketSpecificData {
    fn consume(reader: &mut impl Read) -> io::Result<Self> {
        let mut option_id = 0u8;
        reader.read_exact(bytes_of_mut(&mut option_id))?;
        let mut size = 0u8;
        reader.read_exact(bytes_of_mut(&mut size))?;

        match option_id {
            0 => {
                if size != 4 {
                    Err(io::Error::other(
                        "invalid option size for supported functions",
                    ))
                } else {
                    Ok(Self::SupportedFunctions(reader.read_le_u32()?))
                }
            }
            1 => {
                if size != 16 {
                    Err(io::Error::other(
                        "invalid option size for connection signature",
                    ))
                } else {
                    Ok(Self::ConnectionSignature(
                        reader.read_struct(IS_BIG_ENDIAN)?,
                    ))
                }
            }
            0x80 => {
                if size != 16 {
                    Err(io::Error::other("invalid option size for lite signature"))
                } else {
                    Ok(Self::LiteSignature(reader.read_struct(IS_BIG_ENDIAN)?))
                }
            }
            _ => Err(io::Error::other("invalid option id")),
        }
    }

    fn write_size(&self) -> usize {
        2 + match self {
            PacketSpecificData::SupportedFunctions(_) => 4,
            Self::ConnectionSignature(_) => 16,
            Self::LiteSignature(_) => 16,
        }
    }

    fn write_self(&self, writer: &mut impl Write) -> io::Result<()> {
        match self {
            PacketSpecificData::SupportedFunctions(v) => {
                writer.write_all(&[0, 4])?;
                writer.write_all(&v.to_le_bytes())?;
            }
            Self::ConnectionSignature(v) => {
                writer.write_all(&[1, 16])?;
                writer.write_all(&v[..])?;
            }
            Self::LiteSignature(v) => {
                writer.write_all(&[0x80, 16])?;
                writer.write_all(&v[..])?;
            }
        }

        Ok(())
    }
}

pub struct LitePacket<T: AsRef<[u8]>>(T);

pub struct PacketSpecificIter<'a>(Cursor<&'a [u8]>);

impl<'a> Iterator for PacketSpecificIter<'a> {
    type Item = PacketSpecificData;

    fn next(&mut self) -> Option<Self::Item> {
        PacketSpecificData::consume(&mut self.0).ok()
    }
}

impl<T: AsRef<[u8]>> LitePacket<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn header(&self) -> Option<&LiteHeader> {
        bytemuck::try_from_bytes(self.0.as_ref().get(..size_of::<LiteHeader>())?).ok()
    }
    pub fn header_mut(&mut self) -> Option<&mut LiteHeader>
    where
        T: AsMut<[u8]>,
    {
        bytemuck::try_from_bytes_mut(self.0.as_mut().get_mut(..size_of::<LiteHeader>())?).ok()
    }

    pub fn payload(&self) -> Option<&[u8]> {
        let header = self.header()?;
        self.0
            .as_ref()
            .get(size_of::<LiteHeader>() + header.packet_specific_length as usize..)
    }

    pub fn payload_mut(&mut self) -> Option<&mut [u8]>
    where
        T: AsMut<[u8]>,
    {
        let len = self.header()?.packet_specific_length;
        self.0
            .as_mut()
            .get_mut(size_of::<LiteHeader>() + len as usize..)
    }

    pub fn packet_specific_raw(&self) -> Option<&[u8]> {
        let header = self.header()?;
        self.0.as_ref().get(
            size_of::<LiteHeader>()
                ..size_of::<LiteHeader>() + header.packet_specific_length as usize,
        )
    }
    pub fn packet_specific_raw_mut(&mut self) -> Option<&mut [u8]>
    where
        T: AsMut<[u8]>,
    {
        let len = self.header()?.packet_specific_length;
        self.0
            .as_mut()
            .get_mut(size_of::<LiteHeader>()..size_of::<LiteHeader>() + len as usize)
    }

    pub fn packet_specific_iter<'a>(&'a self) -> Option<PacketSpecificIter<'a>> {
        self.packet_specific_raw()
            .map(Cursor::new)
            .map(PacketSpecificIter)
    }
}

pub fn create_packet_from(
    header: LiteHeader,
    specific_data: &[PacketSpecificData],
    data: &[u8],
) -> Vec<u8> {
    let specific_size: usize = specific_data.iter().map(|v| v.write_size()).sum();
    let mut packet = LitePacket::new(vec![
        0u8;
        size_of::<LiteHeader>() + specific_size + data.len()
    ]);

    *packet.header_mut().expect("packet malformed in creation") = LiteHeader {
        magic: 0x80,
        packet_specific_length: specific_size as u8,
        payload_size: data.len() as u16,
        ..header
    };

    let mut cursor = Cursor::new(
        packet
            .packet_specific_raw_mut()
            .expect("packet malformed in creation"),
    );

    for specific in specific_data {
        specific.write_self(&mut cursor).unwrap();
    }

    packet
        .payload_mut()
        .expect("packet malformed in creation")
        .copy_from_slice(data);

    packet.0
}

#[derive(Pod, Zeroable, Copy, Clone, Default)]
#[repr(transparent)]
pub struct StreamTypes(u8);

impl Debug for StreamTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.source(), self.destination())
    }
}

impl StreamTypes {
    pub fn new(source_stream: u8, dest_stream: u8) -> Self {
        Self((source_stream & 0xF << 4) & dest_stream & 0xF)
    }

    pub fn source(&self) -> u8 {
        self.0 >> 4
    }
    pub fn destination(&self) -> u8 {
        self.0 & 0xF
    }
}
