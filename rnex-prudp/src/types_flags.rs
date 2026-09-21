use std::fmt::{Debug, Formatter};

use bytemuck::{Pod, Zeroable};
use rnex_util::byte::SwapEndian;

#[repr(transparent)]
#[derive(Copy, Clone, Pod, Zeroable, Default, Eq, PartialEq)]
pub struct TypesFlags(pub u16);

impl SwapEndian for TypesFlags {
    fn swap_endian(self) -> Self {
        Self(self.0.swap_bytes())
    }
}

impl TypesFlags {
    #[inline]
    pub const fn get_types(self) -> u8 {
        (self.0 & 0x000F) as u8
    }
    #[inline]
    pub const fn get_flags(self) -> u16 {
        (self.0 & 0xFFF0) >> 4
    }
    #[inline]
    pub const fn types(self, val: u8) -> Self {
        Self((self.0 & 0xFFF0) | (val as u16 & 0x000F))
    }
    #[inline]
    pub const fn flags(self, val: u16) -> Self {
        Self((self.0 & 0x000F) | ((val << 4) & 0xFFF0))
    }
    #[inline]
    pub const fn set_flag(&mut self, val: u16) {
        self.0 |= (val & 0xFFF) << 4;
    }
    #[inline]
    pub const fn set_types(&mut self, val: u8) {
        self.0 |= val as u16 & 0x0F;
    }
}
impl Debug for TypesFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_types();
        let port_number = self.get_flags();
        write!(
            f,
            "TypesFlags{{ types: {}, flags: {} }}",
            stream_type, port_number
        )
    }
}

pub mod flags {
    pub const ACK: u16 = 0x001;
    pub const RELIABLE: u16 = 0x002;
    pub const NEED_ACK: u16 = 0x004;
    pub const HAS_SIZE: u16 = 0x008;
    pub const MULTI_ACK: u16 = 0x200;
}

pub mod types {
    pub const SYN: u8 = 0x0;
    pub const CONNECT: u8 = 0x1;
    pub const DATA: u8 = 0x2;
    pub const DISCONNECT: u8 = 0x3;
    pub const PING: u8 = 0x4;
    /// no idea what user is supposed to mean
    pub const USER: u8 = 0x5;
}
