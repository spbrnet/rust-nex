use macros::{RmcSerialize, method_id, rmc_proto};

use rnex_core::kerberos::KerberosDateTime;
use rnex_core::rmc::structures::qbuffer::QBuffer;
use rnex_core::rmc::structures::resultsrange::ResultsRange;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::ranking::UploadCompetitionData;

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
    pub pid: u32,
    pub score: u32,
    pub modified: KerberosDateTime,
    pub unk2: u8,
    pub appdata: QBuffer,
}

#[rmc_proto(112)]
pub trait Ranking {
    #[method_id(16)]
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode>;
    #[method_id(18)]
    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode>;
}
