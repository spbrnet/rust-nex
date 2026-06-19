use macros::{method_id, rmc_proto};

use crate::rmc::{response::ErrorCode, structures::any::Any};

#[rmc_proto(27, NoReturn)]
pub trait MessageDelivery {
    #[method_id(1)]
    async fn deliver_message(&self, message: Any);
}
