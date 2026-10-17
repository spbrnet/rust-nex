#[cfg(feature = "splatoon")]
use rnex_mm::matchmake::MatchmakeManager;
#[cfg(feature = "splatoon")]
use rnex_server::PassthroughInitModule;
use rnex_server::{ConnectionInitData, RnexManager, RnexModule};

use crate::user::RankingUser;

#[cfg(feature = "splatoon")]
mod results_grpc;
pub mod user;

#[derive(Debug)]
pub struct RankingManager {
    #[cfg(feature = "splatoon")]
    pub(crate) matchmake_manager: PassthroughInitModule<MatchmakeManager>,
}

pub struct RankingModule;

impl RnexManager for RankingManager {
    type User = RankingUser;
    type InitData = ConnectionInitData;

    async fn init_new_user(
        this: rnex_server::PassthroughInitModule<Self>,
        _: &rnex_server::ModuleHolder,
        _: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        _: rnex_server::WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        RankingUser {
            rm: this,
            pid: init_data.pid,
        }
    }
}

impl RnexModule for RankingModule {
    type Manager = RankingManager;
    #[cfg(feature = "splatoon")]
    type InitError = anyhow::Error;
    #[cfg(not(feature = "splatoon"))]
    type InitError = std::convert::Infallible;

    async fn create_manager(
        module_holder: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        #[cfg(feature = "splatoon")]
        {
            let matchmake_manager = module_holder
                .get_ref_init_pt::<MatchmakeManager>()
                .ok_or_else(|| anyhow::anyhow!("splatoon ranking requires matchmaking"))?;
            results_grpc::start(matchmake_manager.results.clone()).await?;
            Ok(RankingManager { matchmake_manager })
        }
        #[cfg(not(feature = "splatoon"))]
        {
            let _ = module_holder;
            Ok(RankingManager {})
        }
    }
}
