use macros::{method_id, rmc_proto, RmcSerialize, rmc_struct};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::qbuffer::QBuffer;

use rnex_core::kerberos::KerberosDateTime;
use rnex_core::PID;

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct PersistenceTarget {
    pub owner: PID,
    pub persistence_slot_id: u16,
}

#[derive(RmcSerialize, Clone)]
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
    pub name: &'static str,
    pub data_type: u16,
    pub meta_binary: QBuffer,
    pub permission: Permission,
    pub del_permission: Permission,
    pub created_time: KerberosDateTime,
    pub updated_time: KerberosDateTime,
    pub period: u16,
    pub status: u8,
    pub reffered_count: u32,
    pub refer_dat_id: u32,
    pub flag: u32,
    pub referred_time: KerberosDateTime,
    pub expire_time: KerberosDateTime,
    pub tags: Vec<&'static str>,
    pub ratings: Vec<RatingInfoWithSlot>,
}

#[rmc_proto(115)]
pub trait DataStore{
    #[method_id(8)]
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode>;
}