use macros::{method_id, rmc_proto};
use rnex_core::prudp::station_url::StationUrl;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::qresult::QResult;

#[rmc_proto(11)]
pub trait Secure {
    #[method_id(1)]
    async fn register(&self, station_urls: Vec<StationUrl>) -> Result<(QResult, u32, StationUrl), ErrorCode>;
    #[method_id(7)]
    async fn replace_url(&self, target: StationUrl, dest: StationUrl) -> Result<(), ErrorCode>;
}
