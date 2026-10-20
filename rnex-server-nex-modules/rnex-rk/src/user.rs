#[cfg(feature = "splatoon1")]
use rnex_mm::results::{ResultReport, SubmitOutcome};
#[cfg(feature = "splatoon1")]
use rnex_rk_protos::ranking::{
    CompetitionRankingGetParam, CompetitionRankingScoreData, CompetitionRankingScoreInfo,
    UploadCompetitionData,
};
use rnex_rk_protos::{LocalRankingProtocol, ranking::Ranking};
use rnex_rmc::rmc_struct;
#[cfg(feature = "splatoon1")]
use rnex_rmc::{qbuffer::QBuffer, response::ErrorCode};
use rnex_server::PassthroughInitModule;
use rnex_util::PID;
#[cfg(feature = "splatoon1")]
use rnex_util::date_time::DateTime;

use crate::RankingManager;

#[cfg(feature = "splatoon2")]
#[derive(Debug, Default)]
pub struct FestivalState {
    pub participants: [std::collections::HashSet<PID>; 2],
    pub votes: [u32; 2],
    pub scores: [u64; 4],
    pub gatherings: std::collections::HashSet<u32>,
}

#[derive(Debug)]
#[rmc_struct(RankingProtocol)]
pub struct RankingUser {
    pub rm: PassthroughInitModule<RankingManager>,
    pub pid: PID,
}

impl Ranking for RankingUser {
    #[cfg(feature = "splatoon2")]
    async fn upload_league_point(
        &self,
        info: rnex_rk_protos::ranking::LeaguePointInfo,
    ) -> Result<(), rnex_rmc::response::ErrorCode> {
        self.rm
            .leagues
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?
            .insert((info.league_id.clone(), info.tag_id), info);
        Ok(())
    }

    #[cfg(feature = "splatoon2")]
    async fn get_league_result(
        &self,
        league_id: String,
        tag_id: u64,
    ) -> Result<rnex_rk_protos::ranking::LeagueResult, rnex_rmc::response::ErrorCode> {
        let leagues = self
            .rm
            .leagues
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let info = leagues
            .get(&(league_id, tag_id))
            .cloned()
            .ok_or(rnex_rmc::response::ErrorCode::RendezVous_InvalidOperation)?;
        let tag_num = info.tag_members.len() as u32;
        Ok(rnex_rk_protos::ranking::LeagueResult {
            league_point_info: info,
            status: 1,
            specific_rank: 1,
            rank_ratio: 100,
            tag_num,
            match_count: 1,
        })
    }

    #[cfg(feature = "splatoon2")]
    async fn upload_x_power(
        &self,
        param: rnex_rk_protos::ranking::XPowerUploadParam,
    ) -> Result<rnex_rk_protos::ranking::XPowerRankingDetail, rnex_rmc::response::ErrorCode> {
        let detail = rnex_rk_protos::ranking::XPowerRankingDetail {
            principal_id: self.pid as u64,
            unique_id: 1_000_000_000_000_000 + self.pid as u64,
            rank: 1,
            last_rank: 1,
            name: param.name,
            region: param.region,
            x_power: param.x_power,
            x_power_max: param.x_power_max,
            weapon_id: param.weapon_id,
            application_buffer: param.application_buffer,
            cheater: 0,
        };
        self.rm
            .x_power
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?
            .insert((param.season_id, self.pid), detail.clone());
        Ok(detail)
    }

    #[cfg(feature = "splatoon2")]
    async fn get_x_power_ranking(
        &self,
        params: Vec<rnex_rk_protos::ranking::XPowerGetRankingParam>,
    ) -> Result<Vec<rnex_rk_protos::ranking::XPowerRanking>, rnex_rmc::response::ErrorCode> {
        let scores = self
            .rm
            .x_power
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let mut results = Vec::with_capacity(params.len());
        for param in params {
            let mut entries: Vec<_> = scores
                .iter()
                .filter(|((season, _), _)| *season == param.season_id)
                .map(|(_, detail)| detail.clone())
                .collect();
            entries.sort_by(|a, b| b.x_power.cmp(&a.x_power));
            for (index, detail) in entries.iter_mut().enumerate() {
                detail.rank = (index + 1) as i32;
            }
            let my_ranking = entries
                .iter()
                .find(|detail| detail.principal_id == self.pid as u64)
                .cloned()
                .unwrap_or_default();
            let weapon_ranking = entries
                .iter()
                .find(|detail| detail.weapon_id == my_ranking.weapon_id)
                .cloned()
                .unwrap_or_default();
            let player_num = entries.len() as i32;
            let top_rankings = entries
                .into_iter()
                .skip(param.top_ranking_offset as usize)
                .take(param.top_ranking_limit.min(1000) as usize)
                .collect();
            results.push(rnex_rk_protos::ranking::XPowerRanking {
                param,
                status: 1,
                player_num,
                top_rankings,
                weapon_ranking,
                my_ranking,
            });
        }
        Ok(results)
    }
    #[cfg(feature = "splatoon2")]
    async fn get_festival_result(
        &self,
        festival_id: u32,
    ) -> Result<rnex_rk_protos::ranking::FestivalResult, rnex_rmc::response::ErrorCode> {
        let festivals = self
            .rm
            .festivals
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let state = festivals.get(&festival_id);
        Ok(rnex_rk_protos::ranking::FestivalResult {
            festival_id,
            num_participants: state.map_or(0, |v| {
                v.participants.iter().map(|team| team.len() as u32).sum()
            }),
            team_participants: state.map_or(vec![0, 0], |v| {
                v.participants
                    .iter()
                    .map(|team| team.len() as u32)
                    .collect()
            }),
            team_scores: vec![0, 0, 0, 0],
            updated_time: rnex_util::date_time::DateTime::now(),
            unknown: 0,
            team_scores_64: state.map_or(vec![0; 4], |v| v.scores.to_vec()),
        })
    }

    #[cfg(feature = "splatoon2")]
    async fn upload_festival_vote(
        &self,
        param: rnex_rk_protos::ranking::FestivalUploadVoteParam,
    ) -> Result<(), rnex_rmc::response::ErrorCode> {
        if param.theme_id > 1 {
            return Err(rnex_rmc::response::ErrorCode::Core_InvalidArgument);
        }
        let mut festivals = self
            .rm
            .festivals
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let state = festivals.entry(param.festival_id).or_default();
        if state.participants[param.theme_id as usize].insert(self.pid) {
            state.votes[param.theme_id as usize] += 1;
        }
        println!("fest vote: {:?}", param);
        Ok(())
    }

    #[cfg(feature = "splatoon2")]
    async fn upload_festival_score(
        &self,
        param: rnex_rk_protos::ranking::FestivalUploadScoreParam,
    ) -> Result<(), rnex_rmc::response::ErrorCode> {
        if param.team_id > 3 {
            return Err(rnex_rmc::response::ErrorCode::Core_InvalidArgument);
        }
        let mut festivals = self
            .rm
            .festivals
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let state = festivals.entry(param.festival_id).or_default();
        if param.battle_gathering_id == 0 || state.gatherings.insert(param.battle_gathering_id) {
            let slot = param.team_id as usize;
            state.scores[slot] = state.scores[slot].saturating_add(param.clout as u64);
        }
        Ok(())
    }

    #[cfg(feature = "splatoon2")]
    async fn delete_festival(&self, festival_id: u32) -> Result<(), rnex_rmc::response::ErrorCode> {
        self.rm
            .festivals
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?
            .remove(&festival_id);
        Ok(())
    }

    #[cfg(feature = "splatoon2")]
    async fn upload_event_match_result(
        &self,
        param: rnex_rk_protos::ranking::EventMatchResult,
    ) -> Result<(), rnex_rmc::response::ErrorCode> {
        let mut matches = self
            .rm
            .event_matches
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        let entries = matches.entry(param.festival_id).or_default();
        if !entries.iter().any(|old| {
            old.event_match_type == param.event_match_type
                && old.players.len() == param.players.len()
                && old.players.iter().zip(&param.players).all(|(a, b)| {
                    a.player_name == b.player_name
                        && a.application_buffer.0 == b.application_buffer.0
                })
        }) {
            entries.insert(0, param);
            entries.truncate(4);
        }
        Ok(())
    }

    #[cfg(feature = "splatoon2")]
    async fn get_event_match_result(
        &self,
        festival_id: u32,
    ) -> Result<Vec<rnex_rk_protos::ranking::EventMatchResult>, rnex_rmc::response::ErrorCode> {
        let matches = self
            .rm
            .event_matches
            .lock()
            .map_err(|_| rnex_rmc::response::ErrorCode::Core_Exception)?;
        Ok(matches.get(&festival_id).cloned().unwrap_or_default())
    }

    #[cfg(feature = "splatoon2")]
    async fn acquire_event_match_right(
        &self,
        _param: rnex_rk_protos::ranking::EventMatchAcquireRightParam,
    ) -> Result<(), rnex_rmc::response::ErrorCode> {
        Ok(())
    }
    #[cfg(feature = "splatoon1")]
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

    #[cfg(feature = "splatoon1")]
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
