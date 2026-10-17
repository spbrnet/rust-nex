#[cfg(feature = "splatoon")]
use rnex_mm::results::{ResultReport, SubmitOutcome};
#[cfg(feature = "splatoon")]
use rnex_rk_protos::ranking::{
    CompetitionRankingGetParam, CompetitionRankingScoreData, CompetitionRankingScoreInfo,
    UploadCompetitionData,
};
use rnex_rk_protos::{LocalRankingProtocol, ranking::Ranking};
use rnex_rmc::rmc_struct;
#[cfg(feature = "splatoon")]
use rnex_rmc::{qbuffer::QBuffer, response::ErrorCode};
use rnex_server::PassthroughInitModule;
use rnex_util::PID;
#[cfg(feature = "splatoon")]
use rnex_util::date_time::DateTime;

use crate::RankingManager;

#[derive(Debug)]
#[rmc_struct(RankingProtocol)]
pub struct RankingUser {
    pub rm: PassthroughInitModule<RankingManager>,
    pub pid: PID,
}

impl Ranking for RankingUser {
    #[cfg(feature = "splatoon")]
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode> {
        let fest_id = param.festival_ids.first().copied().unwrap_or(0);
        let store = &self.rm.matchmake_manager.results;
        let scores = store
            .scores(fest_id, param.range.offset, param.range.size)
            .await
            .map_err(|error| {
                tracing::error!(?error, "could not read splatfest scores");
                ErrorCode::Core_Exception
            })?;
        let team_wins = store.team_wins(fest_id).await.map_err(|error| {
            tracing::error!(?error, "could not read splatfest wins");
            ErrorCode::Core_Exception
        })?;
        let team_votes = store.team_votes(fest_id).await.map_err(|error| {
            tracing::error!(?error, "could not read splatfest votes");
            ErrorCode::Core_Exception
        })?;
        let score_data = scores
            .into_iter()
            .map(|(pid, score)| CompetitionRankingScoreData {
                unk: 1,
                pid,
                score,
                modified: DateTime::now(),
                unk2: 1,
                appdata: QBuffer(vec![]),
            })
            .collect();
        Ok(vec![CompetitionRankingScoreInfo {
            fest_id,
            score_data,
            unk: 0,
            team_wins,
            team_votes,
        }])
    }

    #[cfg(feature = "splatoon")]
    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode> {
        if param.team_id > 1 || param.team_win > 1 {
            return Ok(false);
        }
        let report = ResultReport {
            fest_id: param.splatfest_id,
            pid: self.pid,
            score: param.score,
            team_id: param.team_id,
            team_win: param.team_win == 1,
        };
        match self.rm.matchmake_manager.results.submit(report).await {
            Ok(SubmitOutcome::Stored | SubmitOutcome::Duplicate) => Ok(true),
            Ok(SubmitOutcome::Rejected) => Ok(false),
            Err(error) => {
                tracing::error!(?error, "could not store splatfest result");
                Err(ErrorCode::Core_Exception)
            }
        }
    }
}
