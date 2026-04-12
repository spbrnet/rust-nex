use crate::define_rmc_proto;
use macros::rmc_struct;
use rnex_core::prudp::socket_addr::PRUDPSockAddr;
use std::sync::{Weak};
use rnex_core::PID;
use rnex_core::nex::remote_console::RemoteConsole;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::protocols::datastore::{GetMetaInfo, GetMetaParam};
use rnex_core::rmc::protocols::datastore::{DataStore, RawDataStore, RawDataStoreInfo, RemoteDataStore};

define_rmc_proto!(
    proto DataStoreUser {
        DataStore
    }
);

#[rmc_struct(DataStoreUser)]
pub struct DataStoreUser {
    pub pid: PID,
    pub ip: PRUDPSockAddr,
    pub this: Weak<DataStoreUser>,
    pub remote: RemoteConsole,
}

impl DataStore for DataStoreUser {
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