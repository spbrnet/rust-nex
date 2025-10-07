use std::io;
use std::net::SocketAddrV4;
use macros::{method_id, rmc_proto, RmcSerialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::define_rmc_proto;
use rnex_core::rmc::response::ErrorCode;
use crate::rmc::structures::RmcSerialize;

pub trait UnitPacketRead: AsyncRead + Unpin{
    async fn read_buffer(&mut self) -> Result<Vec<u8>, io::Error>{
        let mut len_raw: [u8; 4] = [0; 4];

        self.read_exact(&mut len_raw).await?;

        let len = u32::from_le_bytes(len_raw);

        let mut vec = vec![0u8; len as _];

        self.read_exact(&mut vec).await?;

        Ok(vec)
    }
}

impl<T: AsyncRead + Unpin> UnitPacketRead for T{}
pub trait UnitPacketWrite: AsyncWrite + Unpin{
    async fn send_buffer(&mut self, data: &[u8]) -> Result<(), io::Error> {
        let mut dest_data = Vec::new();

        data.serialize(&mut dest_data).expect("ran out of memory or something");

        self.write_all(&dest_data[..]).await?;

        self.flush().await?;

        Ok(())
    }
}

impl<T: AsyncWrite + Unpin> UnitPacketWrite for T{}



#[rmc_proto(1)]
pub trait EdgeNodeManagement {
    #[method_id(1)]
    async fn get_url(&self, seed: u64) -> Result<SocketAddrV4, ErrorCode>;
}

define_rmc_proto!(
    proto EdgeNodeHolder{
        EdgeNodeManagement
    }
);

#[derive(RmcSerialize, Debug)]
#[repr(u32)]
pub enum EdgeNodeHolderConnectOption{
    DontRegister = 0,
    Register(SocketAddrV4) = 1
}
