use std::{collections::HashMap, convert::Infallible};

use rnex_msg_protos::RemoteMessagingClient;
use rnex_rmc::{RmcPureRemoteObject, util::PID};
use rnex_server::{ConnectionInitData, RnexManager, RnexModule, WeakPassthroughInitModule};
use tokio::sync::RwLock;

use crate::user::MessagingUser;

pub mod user;

pub struct MessagingManager {
    users_by_pid: RwLock<HashMap<PID, WeakPassthroughInitModule<MessagingUser>>>,
}

pub struct MessagingModule;

impl RnexManager for MessagingManager {
    type User = MessagingUser;

    type InitData = ConnectionInitData;

    async fn init_new_user(
        this: rnex_server::PassthroughInitModule<Self>,
        _mod_holder: &rnex_server::ModuleHolder,
        remote: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        weak_user: rnex_server::WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        this.users_by_pid
            .write()
            .await
            .insert(init_data.pid, weak_user);
        MessagingUser {
            msgm: this,
            pid: init_data.pid,
            remote: RemoteMessagingClient::new(remote.clone()),
        }
    }
}

impl RnexModule for MessagingModule {
    type Manager = MessagingManager;

    type InitError = Infallible;

    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(MessagingManager {
            users_by_pid: Default::default(),
        })
    }
}
