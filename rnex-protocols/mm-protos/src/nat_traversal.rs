use rnex_rmc::{
    method_id,
    response::ErrorCode,
    rmc_proto,
    util::station_url::StationUrl,
};
#[cfg(feature = "nx")]
use rnex_rmc::util::date_time::DateTime;

#[rmc_proto(3)]
pub trait NatTraversal {
    #[cfg(feature = "nx")]
    #[method_id(1)]
    async fn request_probe_initiation(&self, station_to_probe: String) -> Result<(), ErrorCode>;

    #[cfg(not(feature = "nx"))]
    #[method_id(2)]
    async fn request_probe_initiation(&self, station_to_probe: String) -> Result<(), ErrorCode>;

    #[method_id(3)]
    async fn request_probe_initialization_ext(
        &self,
        target_list: Vec<StationUrl>,
        station_to_probe: String,
    ) -> Result<(), ErrorCode>;

    #[method_id(4)]
    async fn report_nat_traversal_result(
        &self,
        cid: u32,
        result: bool,
        rtt: u32,
    ) -> Result<(), ErrorCode>;

    #[method_id(5)]
    async fn report_nat_properties(
        &self,
        nat_mapping: u32,
        nat_filtering: u32,
        rtt: u32,
    ) -> Result<(), ErrorCode>;

    #[cfg(feature = "nx")]
    #[method_id(6)]
    async fn get_relay_signature_key(
        &self,
    ) -> Result<(i32, DateTime, String, u16, i32, u32), ErrorCode>;

    #[cfg(feature = "nx")]
    #[method_id(7)]
    async fn report_nat_traversal_result_detail(
        &self,
        cid: u32,
        result: bool,
        detail: i32,
        rtt: u32,
    ) -> Result<(), ErrorCode>;
}

#[rmc_proto(3, NoReturn)]
pub trait NatTraversalConsole {
    #[method_id(2)]
    async fn request_probe_initiation(&self, station_to_probe: String);
}
