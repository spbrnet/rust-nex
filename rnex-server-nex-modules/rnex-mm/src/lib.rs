use std::{convert::Infallible, sync::atomic::AtomicU32};

use rnex_server::RnexModule;

use crate::matchmake::MatchmakeManager;

pub mod matchmake;
pub mod user;

pub struct MatchMakeModule;
impl RnexModule for MatchMakeModule {
    type Manager = MatchmakeManager;
    type InitError = Infallible;

    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(MatchmakeManager {
            sessions: Default::default(),
            rv_cid_counter: AtomicU32::new(1),
            users: Default::default(),
            users_by_pid: Default::default(),
        })
    }
}
