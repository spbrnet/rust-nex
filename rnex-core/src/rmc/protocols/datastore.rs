use macros::{method_id, rmc_proto, RmcSerialize, rmc_struct};
use rnex_core::rmc::structures::qbuffer::QBuffer;
use rnex_core::rmc::response::ErrorCode;

use rnex_core::kerberos::KerberosDateTime;
use rnex_core::PID;

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct PersistenceTarget {
    pub owner: PID,
    pub persistence_slot_id: u16,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct Permission {
    pub permission: u8,
    pub recipient_ids: Vec<PID>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RatingInfoWithSlot {
    pub slot: i8,
    pub rating: RatingInfo,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RatingInfo {
    pub total_value: i64,
    pub count: u32,
    pub initial_value: i64,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct GetMetaParam {
    pub dataid: u64,
    pub persistence_target: PersistenceTarget,
    pub result_option: u8,
    pub access_password: u64,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct GetMetaInfo {
    pub dataid: u64,
    pub owner: PID,
    pub size: u32,
    pub name: String,
    pub data_type: u16,
    pub meta_binary: QBuffer,
    pub permission: Permission,
    pub del_permission: Permission,
    pub created_time: KerberosDateTime,
    pub updated_time: KerberosDateTime,
    pub period: u16,
    pub status: u8,
    pub referred_count: u32,
    pub refer_dat_id: u32,
    pub flag: u32,
    pub referred_time: KerberosDateTime,
    pub expire_time: KerberosDateTime,
    pub tags: Vec<String>,
    pub ratings: Vec<RatingInfoWithSlot>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RatingInitParam {
    pub flag: u8,
    pub internal_flag: u8,
    pub lock_type: u8,
    pub intial_valie: i64,
    pub range_min: i32,
    pub range_max: i32,
    pub period_hour: i8,
    pub period_duration: i16
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RatingInitParamWithSlot {
    pub slot: i8,
    pub param: RatingInitParam,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct PersistenceInitParam {
    pub persistence_slot_id: u16,
    pub delete_last_object: bool,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct PreparePostParam {
    pub size: u32,
    pub name: String,
    pub data_type: u16,
    pub meta_binary: QBuffer,
    pub permission: Permission,
    pub del_permission: Permission,
    pub flag: u32,
    pub period: u16,
    pub refer_data_id: u32,
    pub tags: Vec<String>,
    pub rating_init_params: Vec<RatingInitParamWithSlot>,
    pub persistence_init_param: PersistenceInitParam,
    pub extra_data: Vec<String>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct ReqPostInfo {
    pub dataid: u64,
    pub url: String,
    pub request_headers: Vec<KeyValue>,
    pub form_fields: Vec<KeyValue>,
    pub root_ca_cert: Vec<u8>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct CompletePostParam {
    pub dataid: u64,
    pub success: bool,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RateCustomRankingParam {
    pub dataid: u64,
    pub appid: u32,
    pub score: u32,
    pub period: u16,
}

#[rmc_proto(115)]
pub trait DataStore{
    #[method_id(8)]
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode>;
    #[method_id(24)]
    async fn prepare_post_object(&self, postparam: PreparePostParam) -> Result<ReqPostInfo, ErrorCode>;
    #[method_id(26)]
    async fn complete_post_object(&self, completeparam: CompletePostParam) -> Result<(), ErrorCode>;
    #[method_id(48)]
    async fn rate_custom_ranking(&self, rankingparam: Vec<RateCustomRankingParam>) -> Result<(), ErrorCode>;
}