use std::net::SocketAddrV4;

use rnex_rmc::{RmcSerialize, define_rmc_proto, method_id, response::ErrorCode, rmc_proto};

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
pub enum EdgeNodeHolderConnectOption {
    DontRegister = 0,
    Register(SocketAddrV4) = 1,
}
