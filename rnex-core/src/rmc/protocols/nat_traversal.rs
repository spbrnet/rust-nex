use macros::{method_id, rmc_proto};
use rnex_core::rmc::response::ErrorCode;

#[rmc_proto(3)]
pub trait NatTraversal{
    #[method_id(2)]
    async fn request_probe_initiation(&self, station_to_probe: String) -> Result<(),ErrorCode>;

    #[method_id(3)]
    async fn request_probe_initialization_ext(&self, target_list: Vec<String>, station_to_probe: String) -> Result<(),ErrorCode>;

    #[method_id(4)]
    async fn report_nat_traversal_result(&self, cid: u32, result: bool, rtt: u32) -> Result<(),ErrorCode>;

    #[method_id(5)]
    async fn report_nat_properties(&self, nat_mapping: u32, nat_filtering: u32, rtt: u32) -> Result<(),ErrorCode>;
}

#[rmc_proto(3, NoReturn)]
pub trait NatTraversalConsole{
    #[method_id(2)]
    async fn request_probe_initiation(&self, station_to_probe: String);
}