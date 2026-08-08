#![cfg(feature = "friends")]
use crate::friends_handler::{FriendsGuest, FriendsUser};
use nex_account::GUEST_PID;
use rnex_fpd_protos::RemoteFriendRemote;
use rnex_rmc::{RmcCallable, RmcPureRemoteObject};
use rnex_server::{
    ConnectionInitData, EnvVarError, RnexManager, RnexModule, WeakPassthroughInitModule, env_var,
};
use rnex_util::PID;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use thiserror::Error;
use tokio::sync::RwLock;
pub mod friends_handler;

#[derive(Error, Debug)]
pub enum ModuleInitError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Env(#[from] EnvVarError),
}

#[derive(Debug)]
pub enum FriendsMaybeGuest {
    Guest(FriendsGuest),
    User(Arc<FriendsUser>),
}

impl RmcCallable for FriendsMaybeGuest {
    async fn rmc_call(
        &self,
        responder: &rnex_util::SendingBufferConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: &[u8],
    ) -> bool {
        match self {
            FriendsMaybeGuest::Guest(friends_guest) => {
                friends_guest
                    .rmc_call(responder, protocol_id, method_id, call_id, rest)
                    .await
            }
            FriendsMaybeGuest::User(friends_user) => {
                friends_user
                    .rmc_call(responder, protocol_id, method_id, call_id, rest)
                    .await
            }
        }
    }
}

#[derive(Debug)]
pub struct FriendsManager {
    pub users: RwLock<HashMap<PID, Weak<FriendsUser>>>,
    pub db: PgPool,
}

pub struct FriendsModule;

impl RnexManager for FriendsManager {
    type InitData = ConnectionInitData;
    type User = FriendsMaybeGuest;

    async fn init_new_user(
        mgr: rnex_server::PassthroughInitModule<Self>,
        _mod_holder: &rnex_server::ModuleHolder,
        remote: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        _weak_user: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        if init_data.pid == GUEST_PID {
            return FriendsMaybeGuest::Guest(FriendsGuest);
        }
        FriendsMaybeGuest::User(Arc::new_cyclic(|this| FriendsUser {
            fm: mgr,
            pid: init_data.pid,
            friend_pids: Default::default(),
            maybe_remote_friend: Default::default(),
            presence: Default::default(),
            this: this.clone(),
            remote: RemoteFriendRemote::new(remote.clone()),
        }))
    }
}

impl RnexModule for FriendsModule {
    type Manager = FriendsManager;

    type InitError = ModuleInitError;

    async fn create_manager(
        _mod_holder: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(FriendsManager {
            users: Default::default(),
            db: PgPool::connect(&env_var("RNEX_DATASTORE_DATABASE")?).await?,
        })
    }
}
