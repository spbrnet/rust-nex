#[allow(async_fn_in_trait)]
pub mod auth_handler;

use nex_account::{grpc::Pid, grpc_client};
use rnex_rmc::util::account::Account;
use rnex_server::{ConnectionInitData, RnexManager, RnexModule, WeakPassthroughInitModule};
use tracing::info;

use crate::auth_handler::AuthHandler;

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
        Self::User { am: this }
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
            // todo: update nex-account to allow pulling the entire account info for rnex
            destination_server_acct: Account::new_raw_key(
                2,
                "Quazal Rendez-Vous",
                grpc_client()
                    .await?
                    .get_nex_key_by_pid(Pid { pid: 2 })
                    .await?
                    .into_inner()
                    .key
                    .try_into()
                    .map_err(|_| anyhow::Error::msg("invalid key size"))?,
            ),
        })
    }
}
