#[allow(unused)]
#[allow(unused_imports)]
use rnex_rmc::response::ErrorCode;
use rnex_server::{ConnectionInitData, EnvVarError, RnexManager, RnexModule, env_var};
use std::str::FromStr;
use cfg_if::cfg_if;
use tracing::error;

use crate::user::RankingUser;

pub mod user;

#[derive(Debug)]
pub struct RankingManager {
    rnex_result_get: String,
    rnex_result_votes_get: String,
    rnex_result_post: String,
}

pub struct RankingModule;

impl RankingManager {
    // Seperate function because I cannot give a fuck right now
    #[cfg(feature = "splatoon")]
    async fn fetch_team_votes(&self, fest_id: u32) -> Result<Vec<u32>, ErrorCode> {
        let url_votes = format!("{}?splatfest_id={}", self.rnex_result_votes_get, fest_id);
        let Ok(response) = tokio::task::spawn_blocking(move || {
            ureq::get(&url_votes).call().map_err(|e| {
                error!("GET for votes failed: {:?}", e);
                ErrorCode::RendezVous_InvalidConfiguration
            })
        })
        .await
        else {
            error!("failed to make request");
            return Err(ErrorCode::Core_Exception);
        };

        let mut response = response?;

        let body = response.body_mut().read_to_string().map_err(|e| {
            error!("failed to read votes body: {:?}", e);
            ErrorCode::RendezVous_InvalidConfiguration
        })?;

        let body = body.trim().trim_start_matches('[').trim_end_matches(']');
        let votes: Result<Vec<u32>, _> = body.split(',').map(|s| u32::from_str(s.trim())).collect();

        votes.map_err(|e| {
            error!("failed to parse votes: {:?}", e);
            ErrorCode::RendezVous_InvalidConfiguration
        })
    }
}

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

    type InitError = EnvVarError;

    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        cfg_if!{
            if #[cfg(feature = "splatoon")] {
                Ok(RankingManager {
                    rnex_result_votes_get: env_var("RNEX_SPLATOON_RESULTS_VOTES_GET")?,
                    rnex_result_post: env_var("RNEX_SPLATOON_RESULTS_POST")?,
                    rnex_result_get: env_var("RNEX_SPLATOON_RESULTS_GET")?,
                })
            } else {
                Ok(RankingManager {
                    rnex_result_get: "".into(),
                    rnex_result_post: "".into(),
                    rnex_result_votes_get: "".into(),
                })
            }
        }
    }
}
