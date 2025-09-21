use macros::{method_id, rmc_proto};
use rnex_core::prudp::station_url::StationUrl;
use rnex_core::rmc::response::ErrorCode;

#[rmc_proto(21)]
pub trait Matchmake{
    #[method_id(2)]
    async fn unregister_gathering(&self, gid: u32) -> Result<bool, ErrorCode>;
    #[method_id(41)]
    async fn get_session_urls(&self, gid: u32) -> Result<Vec<StationUrl>, ErrorCode>;

    #[method_id(42)]
    async fn update_session_host(&self, gid: u32, change_owner: bool) -> Result<(), ErrorCode>;

    #[method_id(44)]
    async fn migrate_gathering_ownership(&self, gid: u32, candidates: Vec<u32>, participants_only: bool) -> Result<(), ErrorCode>;
}