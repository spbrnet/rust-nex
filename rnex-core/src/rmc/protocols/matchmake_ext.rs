use macros::{method_id, rmc_proto};
use rnex_core::rmc::response::ErrorCode;

#[rmc_proto(50)]
pub trait MatchmakeExt{
    #[method_id(1)]
    async fn end_participation(&self, gid: u32, message: String) -> Result<bool, ErrorCode>;
}