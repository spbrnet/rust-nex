use macros::{RmcSerialize, method_id, rmc_proto};

use rnex_core::{
    PID,
    rmc::{response::ErrorCode, structures::any::Any},
};

use crate::{kerberos::KerberosDateTime, rmc::protocols::friends_wiiu::NNAInfo};

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct NintendoCreateAccountData {
    pub nna_info: NNAInfo,
    pub nex_token: String,
    pub birthday: KerberosDateTime,
    pub unk: u64,
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
