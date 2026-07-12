use bytemuck::from_bytes;

pub mod encryption;
pub mod kerberos;
pub mod socket_addr;
pub mod ticket;
pub mod types_flags;
pub mod virtual_port;

fn read_buffer(data: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let len: u32 = *from_bytes(data.get(..4)?);

    let buf = data.get(4..4 + len as usize)?;

    Some((buf.into(), data.get(4 + len as usize..).unwrap_or_default()))
}
