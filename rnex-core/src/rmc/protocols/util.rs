use macros::{method_id, rmc_proto};

use rnex_core::rmc::response::ErrorCode;

#[rmc_proto(110)]
pub trait Utility {
    #[method_id(1)]
    async fn acquire_nex_unique_id(&self) -> Result<u64, ErrorCode>;
}
