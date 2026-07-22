use rnex_rmc::{any::Any, method_id, response::ErrorCode, rmc_proto};

use crate::messaging::UserMessage;

#[rmc_proto(27)]
pub trait MessageDelivery {
    #[method_id(1)]
    async fn deliver_message(&self, message: Any<UserMessage>) -> Result<(), ErrorCode>;
}
#[rmc_proto(27, NoReturn)]
pub trait MessageDeliveryNoResponse {
    #[method_id(1)]
    async fn deliver_message(&self, message: Any<UserMessage>);
}
