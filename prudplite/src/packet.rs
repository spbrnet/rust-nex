use std::fmt;

pub const ACK: u16 = 1;
pub const RELIABLE: u16 = 2;
pub const NEED_ACK: u16 = 4;
pub const HAS_SIZE: u16 = 8;
pub const MULTI_ACK: u16 = 0x200;
pub const SYN: u8 = 0;
pub const CONNECT: u8 = 1;
pub const DATA: u8 = 2;
pub const DISCONNECT: u8 = 3;
pub const PING: u8 = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Packet {
    pub kind: u8,
    pub flags: u16,
    pub source_type: u8,
    pub source_port: u8,
    pub destination_type: u8,
    pub destination_port: u8,
    pub fragment_id: u8,
    pub sequence_id: u16,
    pub signature: Vec<u8>,
    pub connection_signature: Vec<u8>,
    pub minor_version: u8,
    pub supported_functions: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PacketError {
    InvalidMagic,
    InvalidOptions,
    PacketTooLarge,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PacketError {}

impl Packet {
    pub fn encode(&self) -> Result<Vec<u8>, PacketError> {
        if self.payload.len() > u16::MAX as usize {
            return Err(PacketError::PacketTooLarge);
        }
        let mut options = Vec::new();
        if self.kind == SYN || self.kind == CONNECT {
            let support = self.minor_version as u32 | self.supported_functions << 8;
            options.extend_from_slice(&[0, 4]);
            options.extend_from_slice(&support.to_le_bytes());
        }
        let signature = if self.kind == SYN && self.flags & ACK != 0 {
            Some((1, &self.connection_signature))
        } else if self.kind == CONNECT && self.flags & ACK == 0 {
            Some((128, &self.signature))
        } else {
            None
        };
        if let Some((id, value)) = signature {
            if value.len() > u8::MAX as usize {
                return Err(PacketError::InvalidOptions);
            }
            options.extend_from_slice(&[id, value.len() as u8]);
            options.extend_from_slice(value);
        }
        if options.len() > u8::MAX as usize {
            return Err(PacketError::InvalidOptions);
        }
        let mut out = Vec::with_capacity(12 + options.len() + self.payload.len());
        out.extend_from_slice(&[0x80, options.len() as u8]);
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&[
            (self.source_type << 4) | (self.destination_type & 0xf),
            self.source_port,
            self.destination_port,
            self.fragment_id,
        ]);
        out.extend_from_slice(&(self.kind as u16 | self.flags << 4).to_le_bytes());
        out.extend_from_slice(&self.sequence_id.to_le_bytes());
        out.extend_from_slice(&options);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }
}

pub fn decode_packets(input: &[u8]) -> Result<(Vec<Packet>, usize), PacketError> {
    let mut offset = 0;
    let mut packets = Vec::new();
    while input.len() - offset >= 12 {
        let raw = &input[offset..];
        if raw[0] != 0x80 {
            return Err(PacketError::InvalidMagic);
        }
        let options_len = raw[1] as usize;
        let payload_len = u16::from_le_bytes([raw[2], raw[3]]) as usize;
        let total = 12 + options_len + payload_len;
        if raw.len() < total {
            break;
        }
        let type_flags = u16::from_le_bytes([raw[8], raw[9]]);
        let mut packet = Packet {
            kind: (type_flags & 0xf) as u8,
            flags: type_flags >> 4,
            source_type: raw[4] >> 4,
            destination_type: raw[4] & 0xf,
            source_port: raw[5],
            destination_port: raw[6],
            fragment_id: raw[7],
            sequence_id: u16::from_le_bytes([raw[10], raw[11]]),
            payload: raw[12 + options_len..total].to_vec(),
            ..Default::default()
        };
        let options = &raw[12..12 + options_len];
        let mut at = 0;
        while at < options.len() {
            if options.len() - at < 2 {
                return Err(PacketError::InvalidOptions);
            }
            let id = options[at];
            let len = options[at + 1] as usize;
            at += 2;
            if options.len() - at < len {
                return Err(PacketError::InvalidOptions);
            }
            let value = &options[at..at + len];
            match id {
                0 if len == 4 => {
                    let support = u32::from_le_bytes(value.try_into().unwrap());
                    packet.minor_version = support as u8;
                    packet.supported_functions = support >> 8;
                }
                1 => packet.connection_signature = value.to_vec(),
                128 => packet.signature = value.to_vec(),
                _ => {}
            }
            at += len;
        }
        packets.push(packet);
        offset += total;
    }
    Ok((packets, offset))
}