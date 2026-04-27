use macros::{RmcSerialize, method_id, rmc_proto};

use rnex_core::PID;

pub mod notification_types {
    pub const OWNERSHIP_CHANGED: u32 = 4000;
    pub const HOST_CHANGED: u32 = 110000;
    pub const REQUEST_JOIN_GATHERING: u32 = 101;
    pub const END_GATHERING: u32 = 102;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "v3-5-0")]{
#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
pub struct NotificationEvent {
    pub pid_source: PID,
    pub notif_type: u32,
    pub param_1: PID,
    pub param_2: PID,
    pub str_param: String,
    pub param_3: PID,
}
    } else {
        #[derive(RmcSerialize, Debug, Default, Clone)]
        #[rmc_struct(0)]
        pub struct NotificationEvent {
            pub pid_source: PID,
            pub notif_type: u32,
            pub param_1: PID,
            pub param_2: PID,
            pub str_param: String,
        }
    }
}

#[rmc_proto(14, NoReturn)]
pub trait Notification {
    #[method_id(1)]
    async fn process_notification_event(&self, event: NotificationEvent);
}
