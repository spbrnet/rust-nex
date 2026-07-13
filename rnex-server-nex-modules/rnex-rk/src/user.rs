use rnex_rk_protos::{
    LocalRankingProtocol,
    ranking::{
        CompetitionRankingGetParam, CompetitionRankingScoreData, CompetitionRankingScoreInfo,
        Ranking, UploadCompetitionData,
    },
};
use rnex_rmc::{qbuffer::QBuffer, response::ErrorCode, rmc_struct};
use rnex_server::PassthroughInitModule;
use rnex_util::{PID, date_time::DateTime};
use serde::{Deserialize, Serialize};
use std::{env, str::FromStr};
use tracing::{error, info};

use crate::RankingManager;

#[rmc_struct(RankingProtocol)]
pub struct RankingUser {
    pub rm: PassthroughInitModule<RankingManager>,
    pub pid: PID,
}

#[derive(Serialize, Deserialize)]
pub struct CompetitionPostResults {
    pub splatfest_id: u32,
    pub score: u32,
    pub team_id: u8,
    pub team_win: u8,
    pub user: PID,
}

impl Ranking for RankingUser {
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode> {
        let fest_id = param.festival_ids.get(0).copied().unwrap_or(0);

        let url_results = format!("{}?splatfest_id={}", self.rm.rnex_result_get, fest_id);
        let Ok(response_results) =
            tokio::task::spawn_blocking(move || ureq::get(&url_results).call()).await
        else {
            error!("failed to join task");
            return Err(ErrorCode::Core_Exception);
        };

        let results: Vec<CompetitionPostResults> = match response_results {
            Ok(mut res) => res.body_mut().read_json().map_err(|e| {
                error!("failed to parse JSON: {:?}", e);
                ErrorCode::RendezVous_InvalidConfiguration
            })?,
            Err(e) => {
                error!("GET failed: {:?}", e);
                return Err(ErrorCode::RendezVous_InvalidConfiguration);
            }
        };

        let offset = param.range.offset as usize;
        let size = param.range.size as usize;

        let start = offset.min(results.len());
        let end = (start + size).min(results.len());

        let team_votes = self.rm.fetch_team_votes(fest_id).await?;
        let mut wins = vec![0u32, 0u32];
        for r in &results {
            let won_team = (r.team_id ^ (!r.team_win)) & 1;
            if let Some(team) = wins.get_mut(won_team as usize) {
                *team += 1
            };
        }

        let score_data: Vec<CompetitionRankingScoreData> = results[start..end]
            .iter()
            .map(|r| CompetitionRankingScoreData {
                unk: 1,
                pid: r.user,
                score: r.score,
                modified: DateTime::now(),
                unk2: 1,
                appdata: QBuffer(vec![]),
            })
            .collect();

        let info = CompetitionRankingScoreInfo {
            fest_id,
            score_data,
            unk: 0,
            team_wins: wins,
            team_votes,
        };

        println!("range: {:?}", param.range);

        Ok(vec![info])
    }

    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode> {
        info!("fest results for user {:?}:", self.pid);
        info!("fest id: {:?}", param.splatfest_id);
        info!("score: {:?}", param.score);
        info!("team id: {:?}", param.team_id);
        info!("did current team win: {:?}", param.team_win);

        let payload = CompetitionPostResults {
            splatfest_id: param.splatfest_id,
            score: param.score,
            team_id: param.team_id,
            team_win: param.team_win,
            user: self.pid,
        };

        let json_body = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                error!("error making json_body: {:?}", e);
                return Ok(false);
            }
        };

        let rm = self.rm.clone();

        let Ok(response) = tokio::task::spawn_blocking(move || {
            ureq::post(&rm.rnex_result_post)
                .header("Content-Type", "application/json")
                .send(json_body)
        })
        .await
        else {
            error!("unable to spawn blocking");
            return Err(ErrorCode::Core_Exception);
        };

        match response {
            Ok(res) => {
                info!("POST worked: {}", res.status());
            }
            Err(e) => {
                error!("POST borked: {:?}", e);
            }
        }

        Ok(true)
    }
}
