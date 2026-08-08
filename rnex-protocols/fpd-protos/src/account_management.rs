use rnex_rmc::{
    RmcSerialize,
    any::Any,
    data::Data,
    method_id,
    response::ErrorCode,
    rmc_proto,
    util::{PID, date_time::DateTime},
};

use crate::friends_wiiu::NNAInfo;

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct NintendoCreateAccountData {
    #[extends]
    pub data: Data,
    pub nna_info: NNAInfo,
    pub nex_token: String,
    pub birthday: DateTime,
    pub unk: u64,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct AccountExtraInfo {
    #[extends]
    pub data: Data,
    pub unk1: u64,
    pub unk2: u32,
    pub nex_token: String,
}

#[rmc_proto(25)]
pub trait AccountManagement {
    #[method_id(27)]
    async fn nintendo_create_account(
        &self,
        principal_name: String,
        key: String,
        groups: u32,
        email: String,
        auth_data: Any,
    ) -> Result<(PID, String), ErrorCode>;
}
