use std::io::{self, Read};

use bytemuck::Pod;

pub const IS_BIG_ENDIAN: bool = cfg!(target_endian = "big");

pub mod network_endian {
    use std::io::{self, Read};

    #[inline]
    pub fn read_u16(reader: &mut (impl Read + ?Sized)) -> io::Result<u16> {
        let mut bytes = [0_u8; 2];
        reader.read_exact(&mut bytes)?;
        Ok(u16::from_be_bytes(bytes))
    }
}

pub trait ReadExtensions: Read {
    #[inline]
    fn read_le_struct<T: Pod + SwapEndian>(&mut self) -> io::Result<T> {
        self.read_struct(IS_BIG_ENDIAN)
    }

    #[inline]
    fn read_struct<T: Pod + SwapEndian>(&mut self, swap_endian: bool) -> io::Result<T> {
        let mut value = T::zeroed();
        self.read_exact(bytemuck::bytes_of_mut(&mut value))?;
        Ok(if swap_endian {
            value.swap_endian()
        } else {
            value
        })
    }
}

impl<T: Read + ?Sized> ReadExtensions for T {}

pub trait SwapEndian: Copy {
    fn swap_endian(self) -> Self;
}

macro_rules! impl_swap_bytes {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl SwapEndian for $ty {
                #[inline]
                fn swap_endian(self) -> Self {
                    self.swap_bytes()
                }
            }
        )+
    };
}

impl SwapEndian for u8 {
    #[inline]
    fn swap_endian(self) -> Self {
        self
    }
}

impl SwapEndian for i8 {
    #[inline]
    fn swap_endian(self) -> Self {
        self
    }
}

impl_swap_bytes!(u16, u32, u64, i16, i32, i64);

impl SwapEndian for f64 {
    #[inline]
    fn swap_endian(self) -> Self {
        Self::from_bits(self.to_bits().swap_bytes())
    }
}

impl<T: SwapEndian, const SIZE: usize> SwapEndian for [T; SIZE] {
    #[inline]
    fn swap_endian(mut self) -> Self {
        for value in &mut self {
            *value = value.swap_endian();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{ReadExtensions, SwapEndian, network_endian};

    #[test]
    fn reads_structs_in_requested_byte_order() {
        let mut network = Cursor::new([0x12, 0x34]);
        assert_eq!(
            network
                .read_struct::<u16>(cfg!(target_endian = "little"))
                .unwrap(),
            0x1234
        );

        let mut little = Cursor::new([0x34, 0x12]);
        assert_eq!(little.read_le_struct::<u16>().unwrap(), 0x1234);
    }

    #[test]
    fn reads_network_u16() {
        assert_eq!(
            network_endian::read_u16(&mut Cursor::new([0x12, 0x34])).unwrap(),
            0x1234
        );
    }

    #[test]
    fn swaps_arrays_element_by_element() {
        assert_eq!([0x1234_u16, 0x5678].swap_endian(), [0x3412, 0x7856]);
    }
}
