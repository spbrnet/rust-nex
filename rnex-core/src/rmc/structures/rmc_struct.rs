use crate::rmc::structures::Result;
use std::cmp::max;
use std::fmt::Arguments;
use std::io;
use std::io::{ErrorKind, IoSlice, Read, Write};

#[repr(C, packed)]
struct StructureHeader {
    version: u8,
    length: u32,
}

#[cfg(feature = "rmc_struct_header")]
pub const HEADER_SIZE: u32 = 0;
#[cfg(not(feature = "rmc_struct_header"))]
pub const HEADER_SIZE: u32 = 5;

pub struct OnlyWriteVec<'a>(&'a mut Vec<u8>);

impl Write for OnlyWriteVec<'_> {
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }
    fn write_fmt(&mut self, args: Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(args)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }
}

#[cfg(feature = "rmc_struct_header")]
pub fn write_struct<T: Write>(
    writer: &mut T,
    version: u8,
    inner_size: u32,
    pred: impl FnOnce(&mut T) -> Result<()>,
) -> Result<()> {
    use bytemuck::bytes_of;

    writer.write_all(&[version])?;

    writer.write_all(bytes_of(&inner_size))?;

    (pred)(writer)?;

    Ok(())
}

#[cfg(not(feature = "rmc_struct_header"))]
pub fn write_struct<T: Write>(
    writer: &mut T,
    _version: u8,
    _inner_size: u32,
    pred: impl FnOnce(&mut T) -> Result<()>,
) -> Result<()> {
    pred(writer)
}

pub struct SubRead<'a, T: Read> {
    left_to_read: usize,
    origin: &'a mut T,
}

impl<'a, T: Read> SubRead<'a, T> {
    pub const fn new(origin: &'a mut T, left_to_read: usize) -> Self {
        Self {
            left_to_read,
            origin,
        }
    }
}

impl<T: Read> Read for SubRead<'_, T> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let max_read = max(self.left_to_read, buf.len());
        let read = self.origin.read(&mut buf[..max_read])?;
        self.left_to_read -= read;
        Ok(read)
    }

    #[inline(always)]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if buf.len() > self.left_to_read {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "Would run over end of SubRead",
            ));
        }
        self.left_to_read -= buf.len();
        self.origin.read_exact(buf)
    }
}

#[cfg(feature = "rmc_struct_header")]
pub fn read_struct<T: Sized, R: Read>(
    reader: &mut R,
    version: u8,
    pred: impl FnOnce(&mut SubRead<R>) -> Result<T>,
) -> Result<T> {
    use crate::rmc::structures::Error::VersionMismatch;
    use v_byte_helpers::IS_BIG_ENDIAN;
    use v_byte_helpers::ReadExtensions;
    let ver: u8 = reader.read_struct(IS_BIG_ENDIAN)?;

    if ver != version {
        return Err(VersionMismatch(ver));
    }

    let size: u32 = reader.read_struct(IS_BIG_ENDIAN)?;

    Ok(pred(&mut SubRead::new(reader, size as usize))?)
}

#[cfg(not(feature = "rmc_struct_header"))]
pub fn read_struct<T: Sized, R: Read>(
    mut reader: &mut R,
    _version: u8,
    pred: impl FnOnce(&mut R) -> Result<T>,
) -> Result<T> {
    Ok(pred(&mut reader)?)
}
