use macros::{method_id, rmc_proto};

use rnex_core::{
    PID,
    rmc::{response::ErrorCode, structures::any::Any},
};

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
