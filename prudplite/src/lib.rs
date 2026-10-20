#![cfg(feature = "nx")]

mod packet;
mod server;

pub use packet::{Packet, PacketError, decode_packets};
pub use server::start;
