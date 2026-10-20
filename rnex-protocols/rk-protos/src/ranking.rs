use bytemuck::{Pod, Zeroable};
use rnex_base_protos::ResultsRange;
use rnex_rmc::{
    RmcSerialize,
    qbuffer::QBuffer,
    rmc_proto,
    util::{PID, date_time::DateTime},
};
#[cfg(any(feature = "splatoon1", feature = "splatoon2"))]
use rnex_rmc::{method_id, response::ErrorCode};

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct FestivalUploadVoteParam {
    pub festival_id: u32,
    pub theme_id: u8,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(1)]
pub struct FestivalUploadScoreParam {
    pub unique_id: u64,
    pub region: u8,
    pub festival_id: u32,
    pub team_id: u8,
    pub score: u32,
    pub battle_gathering_id: u32,
    pub battle_result: u32,
    pub application_buffer: QBuffer,
    pub clout: u32,
    pub event_match_type: u8,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(1)]
pub struct FestivalResult {
    pub festival_id: u32,
    pub num_participants: u32,
    pub team_participants: Vec<u32>,
    pub team_scores: Vec<u32>,
    pub updated_time: DateTime,
    pub unknown: u8,
    pub team_scores_64: Vec<u64>,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct EventMatchPlayerDetail {
    pub player_name: String,
    pub application_buffer: QBuffer,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct EventMatchResult {
    pub festival_id: u32,
    pub event_match_type: u8,
    pub players: Vec<EventMatchPlayerDetail>,
    pub unknown_number: u64,
    pub unknown_text: String,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct EventMatchAcquireRightParam {
    pub festival_id: u32,
    pub event_match_type: u8,
    pub pids: Vec<PID>,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct LeaguePlayerDetail {
    pub principal_id: u64,
    pub weapon_id: i32,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct LeaguePointInfo {
    pub tag_id: u64,
    pub region_flag: u8,
    pub league_id: String,
    pub point: u32,
    pub tag_members: Vec<(u64, LeaguePlayerDetail)>,
    pub application_buffer: QBuffer,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct LeagueResult {
    pub league_point_info: LeaguePointInfo,
    pub status: u8,
    pub specific_rank: u32,
    pub rank_ratio: u8,
    pub tag_num: u32,
    pub match_count: u32,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct XPowerUploadParam {
    pub season_id: String,
    pub name: String,
    pub region: u8,
    pub x_power: i32,
    pub x_power_max: i32,
    pub weapon_id: i32,
    pub application_buffer: QBuffer,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct XPowerGetRankingParam {
    pub season_id: String,
    pub flag: u8,
    pub top_ranking_offset: u32,
    pub top_ranking_limit: u32,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug, Default)]
#[rmc_struct(0)]
pub struct XPowerRankingDetail {
    pub principal_id: u64,
    pub unique_id: u64,
    pub rank: i32,
    pub last_rank: i32,
    pub name: String,
    pub region: u8,
    pub x_power: i32,
    pub x_power_max: i32,
    pub weapon_id: i32,
    pub application_buffer: QBuffer,
    pub cheater: u32,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct XPowerRanking {
    pub param: XPowerGetRankingParam,
    pub status: u8,
    pub player_num: i32,
    pub top_rankings: Vec<XPowerRankingDetail>,
    pub weapon_ranking: XPowerRankingDetail,
    pub my_ranking: XPowerRankingDetail,
}

#[derive(RmcSerialize, Debug)]
#[rmc_struct(0)]
pub struct UploadCompetitionData {
    pub unk_1: u32,
    pub splatfest_id: u32,
    pub unk_2: u32,
    pub score: u32,
    pub team_id: u8,
    pub team_win: u8,
    pub is_first_upload: bool,
    pub appdata: QBuffer,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct UserData {
    name: [u16; 0x10],
}

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(1)]
pub struct CompetitionRankingGetParam {
    pub unk: u32,
    pub range: ResultsRange,
    pub festival_ids: Vec<u32>,
}

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
pub struct CompetitionRankingScoreInfo {
    pub fest_id: u32,
    pub score_data: Vec<CompetitionRankingScoreData>,
    pub unk: u32,
    pub team_wins: Vec<u32>,
    pub team_votes: Vec<u32>,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct CompetitionRankingScoreData {
    pub unk: u32,
    pub pid: PID,
    pub score: u32,
    pub modified: DateTime,
    pub unk2: u8,
    pub appdata: QBuffer,
}

#[rmc_proto(112)]
pub trait Ranking {
    #[method_id(16)]
    #[cfg(feature = "splatoon2")]
    async fn upload_league_point(&self, info: LeaguePointInfo) -> Result<(), ErrorCode>;
    #[method_id(17)]
    #[cfg(feature = "splatoon2")]
    async fn get_league_result(
        &self,
        league_id: String,
        tag_id: u64,
    ) -> Result<LeagueResult, ErrorCode>;
    #[method_id(18)]
    #[cfg(feature = "splatoon2")]
    async fn get_festival_result(&self, festival_id: u32) -> Result<FestivalResult, ErrorCode>;
    #[method_id(19)]
    #[cfg(feature = "splatoon2")]
    async fn upload_festival_vote(&self, param: FestivalUploadVoteParam) -> Result<(), ErrorCode>;
    #[method_id(20)]
    #[cfg(feature = "splatoon2")]
    async fn upload_festival_score(&self, param: FestivalUploadScoreParam)
    -> Result<(), ErrorCode>;
    #[method_id(21)]
    #[cfg(feature = "splatoon2")]
    async fn delete_festival(&self, festival_id: u32) -> Result<(), ErrorCode>;
    #[method_id(22)]
    #[cfg(feature = "splatoon2")]
    async fn upload_x_power(
        &self,
        param: XPowerUploadParam,
    ) -> Result<XPowerRankingDetail, ErrorCode>;
    #[method_id(23)]
    #[cfg(feature = "splatoon2")]
    async fn get_x_power_ranking(
        &self,
        params: Vec<XPowerGetRankingParam>,
    ) -> Result<Vec<XPowerRanking>, ErrorCode>;
    #[method_id(24)]
    #[cfg(feature = "splatoon2")]
    async fn upload_event_match_result(&self, param: EventMatchResult) -> Result<(), ErrorCode>;
    #[method_id(25)]
    #[cfg(feature = "splatoon2")]
    async fn get_event_match_result(
        &self,
        festival_id: u32,
    ) -> Result<Vec<EventMatchResult>, ErrorCode>;
    #[method_id(26)]
    #[cfg(feature = "splatoon2")]
    async fn acquire_event_match_right(
        &self,
        param: EventMatchAcquireRightParam,
    ) -> Result<(), ErrorCode>;
    #[method_id(16)]
    #[cfg(feature = "splatoon1")]
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode>;
    #[method_id(18)]
    #[cfg(feature = "splatoon1")]
    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode>;
}

#[cfg(all(test, feature = "splatoon2"))]
mod splatoon2_tests {
    use super::*;
    use rnex_rmc::serialization::RmcSerialize as _;

    #[test]
    fn festival_vote_wire_format() {
        let vote = FestivalUploadVoteParam {
            festival_id: 7,
            theme_id: 1,
        };
        assert_eq!(vote.to_data().unwrap(), [0, 5, 0, 0, 0, 7, 0, 0, 0, 1]);
    }
}
