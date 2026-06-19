use macros::{method_id, rmc_proto};

use crate::rmc::{
    protocols::messaging::UserMessage,
    response::ErrorCode,
    structures::{Error, any::Any},
};

#[rmc_proto(27)]
pub trait MessageDelivery {
    #[method_id(1)]
    async fn deliver_message(&self, message: Any<UserMessage>) -> Result<(), ErrorCode>;
}
// #[rmc_proto(27, NoReturn)]
// pub trait MessageDeliveryNoResponse {
//     #[method_id(1)]
//     async fn deliver_message(&self, message: Any<UserMessage>);
// }
