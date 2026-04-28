use macros::{RmcSerialize, method_id, rmc_proto};

use rnex_core::{
    PID,
    rmc::{response::ErrorCode, structures::any::Any},
};

use crate::{kerberos::KerberosDateTime, rmc::protocols::friends::NNAInfo};

#[rmc_proto(110)]
pub trait Utility {
    #[method_id(1)]
    async fn acquire_nex_unique_id(&self) -> Result<u64, ErrorCode>;
}
