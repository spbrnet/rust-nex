#![allow(async_fn_in_trait)]

use rnex_rmc::define_rmc_proto;

pub mod ranking;
use ranking::{Ranking, RawRanking, RawRankingInfo, RemoteRanking};

define_rmc_proto!(
    proto RankingProtocol{
        Ranking,
    }
);
