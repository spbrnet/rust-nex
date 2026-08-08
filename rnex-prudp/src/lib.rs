pub mod encryption;
pub mod kerberos;
pub mod socket_addr;
pub mod ticket;
pub mod types_flags;
pub mod virtual_port;

fn read_buffer(data: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let len_bytes: [u8; 4] = data.get(..4)?.try_into().ok()?;

    let len = u32::from_ne_bytes(len_bytes) as usize;

    let buf = data.get(4..4 + len)?;

    Some((buf.to_vec(), data.get(4 + len..).unwrap_or_default()))
}
