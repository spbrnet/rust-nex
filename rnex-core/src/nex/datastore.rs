use rnex_core::rmc::protocols::datastore::DataStore;
// use crate::define_rmc_proto;
// use macros::rmc_struct;
use rnex_core::prudp::socket_addr::PRUDPSockAddr;
use std::sync::{Weak};
use rnex_core::PID;
use crate::nex::remote_console::RemoteConsole;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::protocols::datastore::{GetMetaInfo, GetMetaParam};

pub struct User {
    pub pid: PID,
    pub ip: PRUDPSockAddr,
    pub this: Weak<User>,
    pub remote: RemoteConsole,
}

impl DataStore for User {
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode> {
        // // bogus
        // let info: GetMetaInfo = GetMetaInfo {
        //     dataid: 10,
        //     owner: 1121,
        //     size: 1024,
        //     name: "idk"
        // }

        // just trying to see what methods it tries to use
        Err(ErrorCode::DataStore_NotFound)
    }
}