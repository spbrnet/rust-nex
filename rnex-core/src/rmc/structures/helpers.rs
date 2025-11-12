use std::{fmt, io};

pub struct DummyFormatWriter(u32);

impl fmt::Write for DummyFormatWriter{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += s.as_bytes().len() as u32;
        Ok(())
    }
}

impl DummyFormatWriter{
    pub const fn new() -> Self{ Self(0) }
    pub const fn serialize_str_len(&self) -> u32 {
        2 + self.0 + 1
    }
}

pub struct DummyWriter(u32);

impl io::Write for DummyWriter{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len() as u32;
        Ok(buf.len())
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0 += buf.len() as u32;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl DummyWriter{
    pub const fn new() -> Self{ Self(0) }
    pub const fn get_total_len(&self) -> u32{
        self.0
    }
}


pub fn len_of_write(f: impl FnOnce(&mut DummyWriter) -> anyhow::Result<()>) -> u32{
    let mut dummy = DummyWriter::new();
    f(&mut dummy).ok();
    dummy.get_total_len()
}