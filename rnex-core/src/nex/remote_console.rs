use crate::rmc::protocols::notifications::{Notification, RawNotification, RawNotificationInfo, RemoteNotification};
use crate::rmc::protocols::nat_traversal::{NatTraversalConsole, RemoteNatTraversalConsole, RawNatTraversalConsoleInfo, RawNatTraversalConsole};
use crate::define_rmc_proto;

define_rmc_proto!(
    proto Console{
        Notification,
        NatTraversalConsole
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