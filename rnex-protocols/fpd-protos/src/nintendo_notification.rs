use rnex_rmc::{RmcSerialize, any::Any, data::Data, method_id, rmc_proto, util::PID};

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct NintendoNotificationEvent {
    pub event_type: u32,
    pub sender: PID,
    pub data: Any,
}

#[derive(RmcSerialize, Default)]
#[rmc_struct(0)]
pub struct NintendoNotificationEventGeneral {
    #[extends]
    pub data: Data,
    pub param1: u32,
    pub param2: u64,
    pub param3: u64,
    pub str_param: String,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct NintendoNotificationEventProfile {
    #[extends]
    pub data: Data,
    pub region: u8,
    pub country: u8,
    pub area: u8,
    pub language: u8,
    pub platform: u8,
}

#[rmc_proto(100, NoReturn)]
pub trait NintendoNotification {
    #[method_id(1)]
    async fn process_nintendo_notification_event_1(&self, notif: NintendoNotificationEvent);
    #[method_id(2)]
    async fn process_nintendo_notification_event_2(&self, notif: NintendoNotificationEvent);
}
