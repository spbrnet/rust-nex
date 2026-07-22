use std::{
    convert::Infallible,
    sync::atomic::{AtomicU32, Ordering::Relaxed},
};

use rnex_server::{ConnectionInitData, RnexManager, RnexModule, WeakPassthroughInitModule};

use crate::user::BaseUser;

pub mod user;

#[derive(Default, Debug)]
pub struct BaseManager {
    cid_counter: AtomicU32,
}

#[derive(Debug, Default)]
pub struct BaseModule;

impl RnexManager for BaseManager {
    type User = BaseUser;

    type InitData = ConnectionInitData;

    async fn init_new_user(
        this: rnex_server::PassthroughInitModule<Self>,
        _mod_holder: &rnex_server::ModuleHolder,
        _remote: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        _: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        BaseUser {
            cid: this.cid_counter.fetch_add(1, Relaxed),
            //bm: this,
            addr: init_data.addr,
            pid: init_data.pid,
            station_url: Default::default(),
        }
    }
}

impl RnexModule for BaseModule {
    type Manager = BaseManager;

    type InitError = Infallible;

    async fn create_manager(
        _mod_holder: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(BaseManager::default())
    }
}
