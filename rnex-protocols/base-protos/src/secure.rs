use rnex_rmc::{
    RmcSerialize, any::Any, data::Data, method_id, qbuffer::QBuffer, qresult::QResult, response::ErrorCode, rmc_proto, util::station_url::StationUrl
};

#[derive(RmcSerialize)]
#[rmc_struct(0)]
struct NintendoLoginData {
    #[extends]
    data: Data,
    token: String,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
struct AccountExtraInfo {
    #[extends]
    data: Data,
    token: String,
}

#[rmc_proto(11)]
pub trait Secure {
    #[method_id(1)]
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode>;

    #[method_id(4)]
    async fn register_ex(
        &self,
        station_urls: Vec<StationUrl>,
        data: Any,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode>;
    #[method_id(7)]
    async fn replace_url(&self, target: StationUrl, dest: StationUrl) -> Result<(), ErrorCode>;
    #[method_id(8)]
    async fn send_report(&self, id: u32, data: QBuffer) -> Result<(), ErrorCode>;
}
