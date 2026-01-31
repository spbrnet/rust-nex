use std::mem::transmute;

use bytemuck::{Pod, Zeroable, try_from_bytes, try_from_bytes_mut};
use log::{error, info};
use rnex_core::prudp::{
    types_flags::{
        self, TypesFlags,
        flags::{HAS_SIZE, NEED_ACK},
        types::{CONNECT, DATA, PING, SYN},
    },
    virtual_port::VirtualPort,
};

use crate::crypto::{Crypto, CryptoInstance};

#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct PRUDPV0Header {
    pub source: VirtualPort,
    pub destination: VirtualPort,
    pub type_flags: TypesFlags,
    pub session_id: u8,
    pub packet_signature: [u8; 4],
    pub sequence_id: u16,
}
#[repr(transparent)]
pub struct PRUDPV0Packet<T: AsRef<[u8]>>(pub T);

impl<T: AsRef<[u8]>> PRUDPV0Packet<T> {
    #[inline(always)]
    pub fn get_packet_specific_size(&self) -> Option<usize> {
        Some(get_types_flags_size_from_types_flags(
            self.header()?.type_flags,
        ))
    }

    #[inline(always)]
    pub fn header(&self) -> Option<&PRUDPV0Header> {
        try_from_bytes(self.0.as_ref().get(..size_of::<PRUDPV0Header>())?).ok()
    }
    #[inline(always)]
    pub fn header_mut(&mut self) -> Option<&mut PRUDPV0Header>
    where
        T: AsMut<[u8]>,
    {
        try_from_bytes_mut(self.0.as_mut().get_mut(..size_of::<PRUDPV0Header>())?).ok()
    }

    #[inline(always)]
    pub fn connection_signature(&self) -> Option<&[u8; 4]> {
        let offset = size_of::<PRUDPV0Header>();
        Some(self.0.as_ref().get(offset..offset + 4)?.try_into().ok()?)
    }
    #[inline(always)]
    pub fn connection_signature_mut(&mut self) -> Option<&mut [u8; 4]>
    where
        T: AsMut<[u8]>,
    {
        let offset = size_of::<PRUDPV0Header>();
        Some(
            self.0
                .as_mut()
                .get_mut(offset..offset + 4)?
                .try_into()
                .ok()?,
        )
    }
    #[inline(always)]
    pub fn size_mut(&mut self) -> Option<&mut [u8]>
    where
        T: AsMut<[u8]>,
    {
        if self.header()?.type_flags.get_flags() & HAS_SIZE == 0 {
            return None;
        }
        let offset = size_of::<PRUDPV0Header>() + get_type_specific_size(self.header()?.type_flags);
        Some(self.0.as_mut().get_mut(offset..offset + 2)?)
    }

    #[inline(always)]
    pub fn fragment_id_mut(&mut self) -> Option<&mut u8>
    where
        T: AsMut<[u8]>,
    {
        if self.header()?.type_flags.get_types() != DATA {
            return None;
        }
        let offset = size_of::<PRUDPV0Header>();
        Some(self.0.as_mut().get_mut(offset)?)
    }

    #[inline(always)]
    pub fn fragment_id(&self) -> Option<&u8> {
        if self.header()?.type_flags.get_types() != DATA {
            return None;
        }
        let offset = size_of::<PRUDPV0Header>();
        Some(self.0.as_ref().get(offset)?)
    }

    #[inline(always)]
    fn get_payload_offset(&self) -> Option<usize> {
        Some(size_of::<PRUDPV0Header>() + self.get_packet_specific_size()?)
    }

    #[inline(always)]
    pub fn payload(&self) -> Option<&[u8]> {
        self.0
            .as_ref()
            .get(self.get_payload_offset()?..(self.0.as_ref().len().saturating_sub(1)))
    }
    #[inline(always)]
    pub fn payload_mut(&mut self) -> Option<&mut [u8]>
    where
        T: AsMut<[u8]>,
    {
        let start_offset = self.get_payload_offset()?;
        let end_offset = self.0.as_ref().len().saturating_sub(1);
        self.0.as_mut().get_mut(start_offset..end_offset)
    }

    #[inline(always)]
    pub fn checksummed_data(&self) -> Option<&[u8]> {
        self.0
            .as_ref()
            .get(..self.0.as_ref().len().saturating_sub(1))
    }

    #[inline(always)]
    pub fn checksum(&self) -> Option<u8> {
        self.0.as_ref().last().copied()
    }
    #[inline(always)]
    pub fn checksum_mut(&mut self) -> Option<&mut u8>
    where
        T: AsMut<[u8]>,
    {
        self.0.as_mut().last_mut()
    }

    #[inline(always)]
    pub fn check_checksum(&self, crypto: &impl Crypto) -> bool {
        let Some(data) = self.checksummed_data() else {
            return false;
        };
        let Some(checksum) = self.checksum() else {
            return false;
        };
        checksum == crypto.calculate_checksum(data)
    }

    pub fn new(data: T) -> Self {
        Self(data)
    }
}

const DEFAULT_SIGNAT: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
#[inline(always)]
const fn get_size_offset(tf: TypesFlags) -> usize {
    size_of::<PRUDPV0Header>()
        + (if tf.get_types() & (SYN | CONNECT) != 0 {
            4
        } else if tf.get_types() & DATA != 0 {
            1
        } else {
            0
        })
}
#[inline(always)]
const fn get_type_specific_size(tf: TypesFlags) -> usize {
    if tf.get_types() == SYN || tf.get_types() == CONNECT {
        4
    } else if tf.get_types() & DATA != 0 {
        1
    } else {
        0
    }
}
#[inline(always)]
const fn get_types_flags_size_from_types_flags(tf: TypesFlags) -> usize {
    get_type_specific_size(tf) + (if tf.get_flags() & HAS_SIZE != 0 { 2 } else { 0 })
}
#[inline(always)]
pub const fn precalc_size(tf: TypesFlags, payload_size: usize) -> usize {
    size_of::<PRUDPV0Header>() + get_types_flags_size_from_types_flags(tf) + payload_size + 1
}
pub fn new_syn_packet(
    flags: u16,
    source: VirtualPort,
    destination: VirtualPort,
    signat: [u8; 4],
    crypto: &impl Crypto,
) -> Vec<u8> {
    let type_flags = TypesFlags::default().types(SYN).flags(flags);

    let vec = vec![0; precalc_size(type_flags, 0)];
    let mut packet = PRUDPV0Packet::new(vec);
    let header = packet.header_mut().expect("packet malformed in creation");

    *header = PRUDPV0Header {
        destination,
        source,
        packet_signature: DEFAULT_SIGNAT,
        sequence_id: 0,
        session_id: 0,
        type_flags,
    };
    *packet
        .connection_signature_mut()
        .expect("packet malformed in creation") = signat;

    *packet.checksum_mut().expect("packet malformed in creation") = crypto.calculate_checksum(
        packet
            .checksummed_data()
            .expect("packet malformed in creation"),
    );

    packet.0
}

pub fn new_connect_packet(
    flags: u16,
    source: VirtualPort,
    destination: VirtualPort,
    self_signat: [u8; 4],
    remote_signat: [u8; 4],
    session_id: u8,
    data: &[u8],
    crypto: &impl Crypto,
) -> Vec<u8> {
    let type_flags = TypesFlags::default().types(CONNECT).flags(flags);

    let vec = vec![0; precalc_size(type_flags, 0)];
    let mut packet = PRUDPV0Packet::new(vec);
    let header = packet.header_mut().expect("packet malformed in creation");

    *header = PRUDPV0Header {
        destination,
        source,
        packet_signature: self_signat,
        sequence_id: 1,
        session_id,
        type_flags,
    };
    *packet
        .connection_signature_mut()
        .expect("packet malformed in creation") = remote_signat;
    if let Some(size) = packet.size_mut() {
        size.copy_from_slice(&(data.len() as u16).to_le_bytes());
    }
    *packet.checksum_mut().expect("packet malformed in creation") = crypto.calculate_checksum(
        packet
            .checksummed_data()
            .expect("packet malformed in creation"),
    );

    packet.0
}

pub fn new_data_packet(
    flags: u16,
    source: VirtualPort,
    destination: VirtualPort,
    data: &[u8],
    sequence_id: u16,
    session_id: u8,
    frag_id: u8,
    crypto_instance: &mut impl CryptoInstance,
    crypto: &impl Crypto,
) -> Vec<u8> {
    let type_flags = TypesFlags::default().types(DATA).flags(flags);

    let vec = vec![0; precalc_size(type_flags, data.len())];
    let mut packet = PRUDPV0Packet::new(vec);
    packet
        .header_mut()
        .expect("packet malformed in creation")
        .type_flags = type_flags;
    packet
        .payload_mut()
        .expect("packet malformed in creation")
        .copy_from_slice(data);

    if let Some(size) = packet.size_mut() {
        size.copy_from_slice(&(data.len() as u16).to_le_bytes());
    }
    *packet
        .fragment_id_mut()
        .expect("packet malformed in creation") = frag_id;
    let packet_signature = crypto_instance.generate_signature(
        type_flags,
        packet.payload().expect("packet malformed in creation"),
    );

    let header = packet.header_mut().expect("packet malformed in creation");

    *header = PRUDPV0Header {
        destination,
        source,
        packet_signature,
        sequence_id,
        session_id,
        type_flags,
    };

    *packet.checksum_mut().expect("packet malformed in creation") = crypto.calculate_checksum(
        packet
            .checksummed_data()
            .expect("packet malformed in creation"),
    );

    info!("header: {:?}", packet.header());

    packet.0
}

pub fn new_ping_packet(
    flags: u16,
    source: VirtualPort,
    destination: VirtualPort,
    sequence_id: u16,
    session_id: u8,
    crypto_instance: &mut impl CryptoInstance,
    crypto: &impl Crypto,
) -> Vec<u8> {
    let type_flags = TypesFlags::default().types(PING).flags(flags);

    let vec = vec![0; precalc_size(type_flags, 0)];
    let mut packet = PRUDPV0Packet::new(vec);

    let packet_signature = crypto_instance.generate_signature(type_flags, &[]);

    let header = packet.header_mut().expect("packet malformed in creation");

    *header = PRUDPV0Header {
        destination,
        source,
        packet_signature,
        sequence_id,
        session_id,
        type_flags,
    };

    *packet.checksum_mut().expect("packet malformed in creation") = crypto.calculate_checksum(
        packet
            .checksummed_data()
            .expect("packet malformed in creation"),
    );

    info!("header: {:?}", packet.header());

    packet.0
}
