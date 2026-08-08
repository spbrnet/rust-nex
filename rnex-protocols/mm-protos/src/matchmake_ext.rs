use rnex_rmc::{method_id, response::ErrorCode, rmc_proto};

#[rmc_proto(50)]
pub trait MatchmakeExt {
    #[method_id(1)]
    async fn end_participation(&self, gid: u32, message: String) -> Result<bool, ErrorCode>;
}
