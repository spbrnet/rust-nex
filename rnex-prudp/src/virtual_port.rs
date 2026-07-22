use bytemuck::{Pod, Zeroable};
use std::fmt::{Debug, Formatter};
use v_byte_helpers::SwapEndian;

#[repr(transparent)]
#[derive(PartialEq, Eq, Ord, PartialOrd, Copy, Clone, Pod, Zeroable, SwapEndian, Hash, Default)]
pub struct VirtualPort(pub u8);

impl VirtualPort {
    #[inline]
    pub const fn get_stream_type(self) -> u8 {
        (self.0 & 0xF0) >> 4
    }

    #[inline]
    pub const fn get_port_number(self) -> u8 {
        self.0 & 0x0F
    }

    #[inline]
    pub fn stream_type(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0x0F) | (masked_val << 4))
    }

    #[inline]
    pub fn port_number(self, val: u8) -> Self {
        let masked_val = val & 0x0F;
        assert_eq!(masked_val, val);

        Self((self.0 & 0xF0) | masked_val)
    }

    #[inline]
    pub fn new(port: u8, stream_type: u8) -> Self {
        Self(0).stream_type(stream_type).port_number(port)
    }
    #[inline(always)]
    pub fn parse(data: &str) -> Option<Self> {
        let (p1, p2) = data.split_once(':')?;
        Some(Self::new(p1.parse().ok()?, p2.parse().ok()?))
    }
}

impl Debug for VirtualPort {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let stream_type = self.get_stream_type();
        let port_number = self.get_port_number();
        write!(
            f,
            "VirtualPort{{ stream_type: {}, port_number: {} }}",
            stream_type, port_number
        )
    }
}
