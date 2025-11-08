use macros::rmc_proto;
use crate::rmc::response::ErrorCode;
use macros::{method_id, rmc_struct, RmcSerialize};

#[derive(RmcSerialize, Debug, Default, Clone)]
struct ResultsRange{
    offset: u32,
    size: u32
}

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(1)]
struct CompetitionRankingGetParam{
    unk: u32,
    range: ResultsRange,
    festival_ids: Vec<u32>,
}

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
struct CompetitionRankingScoreInfo{
    fest_id: u32,
    score_data: Vec<u32>,
    unk: u32,
    team_wins: Vec<u32>,
    team_votes: Vec<u32>
}



#[rmc_proto(112)]
pub trait Ranking{
    //#[method_id(16)]
    //async fn competition_ranking_get_param(&self, param: CompetitionRankingGetParam) -> Result<Vec<CompetitionRankingScoreInfo>,ErrorCode>;
}