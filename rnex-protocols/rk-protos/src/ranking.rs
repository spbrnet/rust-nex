use bytemuck::{Pod, Zeroable};
use rnex_base_protos::ResultsRange;
use rnex_rmc::{
    RmcSerialize, method_id,
    qbuffer::QBuffer,
    response::ErrorCode,
    rmc_proto,
    util::{PID, date_time::DateTime},
};

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
    #[cfg(feature = "splatoon")]
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode>;
    #[method_id(18)]
    #[cfg(feature = "splatoon")]
    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode>;
}
