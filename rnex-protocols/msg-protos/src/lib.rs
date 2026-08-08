#![allow(async_fn_in_trait)]

pub mod message_delivery;
pub mod messaging;
use message_delivery::{
    MessageDelivery, MessageDeliveryNoResponse, RawMessageDelivery, RawMessageDeliveryInfo,
    RawMessageDeliveryNoResponse, RawMessageDeliveryNoResponseInfo, RemoteMessageDelivery,
    RemoteMessageDeliveryNoResponse,
};
use rnex_rmc::define_rmc_proto;

define_rmc_proto!(
    proto MessagingProtocol{
        MessageDelivery
    }
);

define_rmc_proto!(
    proto MessagingClient{
        MessageDeliveryNoResponse
    }
);
