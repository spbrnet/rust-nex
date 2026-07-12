use crate::define_rmc_proto;
use crate::rmc::protocols::message_delivery::{
    MessageDeliveryNoResponse, RawMessageDeliveryNoResponse, RawMessageDeliveryNoResponseInfo,
    RemoteMessageDeliveryNoResponse,
};
use crate::rmc::protocols::nat_traversal::{
    NatTraversalConsole, RawNatTraversalConsole, RawNatTraversalConsoleInfo,
    RemoteNatTraversalConsole,
};
use crate::rmc::protocols::notifications::{
    Notification, RawNotification, RawNotificationInfo, RemoteNotification,
};

define_rmc_proto!(
    proto Console{
        Notification,
        NatTraversalConsole,
        MessageDeliveryNoResponse
    }
);
/*
#[rmc_struct(Console)]
pub struct TestRemoteConsole{
    pub remote: RemoteUserProtocol,
}

impl Notification for TestRemoteConsole{
    async fn process_notification_event(&self, event: NotificationEvent) {
        println!("NOTIF RECIEVED: {:?}", event);
    }
}*/
