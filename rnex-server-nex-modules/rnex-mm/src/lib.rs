#[cfg(not(feature = "splatoon1"))]
use std::convert::Infallible;
use std::sync::atomic::AtomicU32;

use rnex_server::RnexModule;

use crate::matchmake::MatchmakeManager;

pub mod matchmake;
#[cfg(feature = "splatoon1")]
pub mod results;
pub mod user;

pub struct MatchMakeModule;
impl RnexModule for MatchMakeModule {
    type Manager = MatchmakeManager;
    #[cfg(not(feature = "splatoon1"))]
    type InitError = Infallible;
    #[cfg(feature = "splatoon1")]
    type InitError = anyhow::Error;

    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        #[cfg(feature = "splatoon1")]
        let results = results::ResultsStore::connect().await?;
        Ok(MatchmakeManager {
            sessions: Default::default(),
            rv_cid_counter: AtomicU32::new(1),
            users: Default::default(),
            users_by_pid: Default::default(),
            #[cfg(feature = "splatoon1")]
            results,
        })
    }
}
