#[allow(async_fn_in_trait)]
pub mod auth_handler;

use rnex_rmc::util::account::Account;
use rnex_server::{ConnectionInitData, RnexManager, RnexModule, WeakPassthroughInitModule};
use tracing::info;

use crate::auth_handler::AuthHandler;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::RwLock;

static MAINTENANCE: OnceLock<Arc<AtomicBool>> = OnceLock::new();

pub fn maintenance_flag() -> Arc<AtomicBool> {
    MAINTENANCE
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

pub fn set_maintenance(on: bool) {
    maintenance_flag().store(on, Ordering::Relaxed);
}

pub fn is_maintenance() -> bool {
    maintenance_flag().load(Ordering::Relaxed)
}

#[derive(Debug)]
pub struct AuthManager {
    pub destination_server_acct: Account,
    pub build_name: &'static str,
}
#[derive(Debug)]
pub struct AuthModule;

impl RnexManager for AuthManager {
    type User = AuthHandler;
    type InitData = ConnectionInitData;
    async fn init_new_user(
        this: rnex_server::PassthroughInitModule<Self>,
        _: &rnex_server::ModuleHolder,
        _: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        _: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        info!(target: "proxy_connections", address = ?init_data, "user connected");
        Self::User {
            am: this,
            authenticated_account: RwLock::new(None),
        }
    }
}

impl RnexModule for AuthModule {
    type Manager = AuthManager;
    type InitError = anyhow::Error;
    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(AuthManager {
            build_name: option_env!("AUTH_REPORT_VERSION").unwrap_or("no version specified"),
            destination_server_acct: Account::from_password_env(
                2,
                "Quazal Rendez-Vous",
                "RNEX_SERVER_PASSWORD",
            )?,
        })
    }
}
