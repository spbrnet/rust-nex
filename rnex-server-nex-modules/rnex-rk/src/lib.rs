#[cfg(feature = "splatoon1")]
use rnex_mm::matchmake::MatchmakeManager;
#[cfg(feature = "splatoon1")]
use rnex_server::PassthroughInitModule;
use rnex_server::{ConnectionInitData, RnexManager, RnexModule};

use crate::user::RankingUser;

#[cfg(feature = "splatoon1")]
mod results_grpc;
pub mod user;

#[derive(Debug)]
pub struct RankingManager {
    #[cfg(feature = "splatoon2")]
    pub(crate) festivals: std::sync::Mutex<std::collections::HashMap<u32, user::FestivalState>>,
    #[cfg(feature = "splatoon2")]
    pub(crate) event_matches: std::sync::Mutex<
        std::collections::HashMap<u32, Vec<rnex_rk_protos::ranking::EventMatchResult>>,
    >,
    #[cfg(feature = "splatoon2")]
    pub(crate) leagues: std::sync::Mutex<
        std::collections::HashMap<(String, u64), rnex_rk_protos::ranking::LeaguePointInfo>,
    >,
    #[cfg(feature = "splatoon2")]
    pub(crate) x_power: std::sync::Mutex<
        std::collections::HashMap<
            (String, rnex_util::PID),
            rnex_rk_protos::ranking::XPowerRankingDetail,
        >,
    >,
    #[cfg(feature = "splatoon1")]
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
    #[cfg(feature = "splatoon1")]
    type InitError = anyhow::Error;
    #[cfg(not(feature = "splatoon1"))]
    type InitError = std::convert::Infallible;

    async fn create_manager(
        module_holder: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        #[cfg(feature = "splatoon1")]
        {
            let matchmake_manager = module_holder
                .get_ref_init_pt::<MatchmakeManager>()
                .ok_or_else(|| anyhow::anyhow!("splatoon1 ranking requires matchmaking"))?;
            results_grpc::start(matchmake_manager.results.clone()).await?;
            Ok(RankingManager { matchmake_manager })
        }
        #[cfg(not(feature = "splatoon1"))]
        {
            let _ = module_holder;
            Ok(RankingManager {
                #[cfg(feature = "splatoon2")]
                festivals: Default::default(),
                #[cfg(feature = "splatoon2")]
                event_matches: Default::default(),
                #[cfg(feature = "splatoon2")]
                leagues: Default::default(),
                #[cfg(feature = "splatoon2")]
                x_power: Default::default(),
            })
        }
    }
}
