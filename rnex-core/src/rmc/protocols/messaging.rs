use macros::RmcSerialize;

use crate::{
    kerberos::KerberosDateTime,
    rmc::structures::{data::Data, qbuffer::QBuffer},
};

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct UserMessage {
    #[extends]
    pub data: Data,
    pub id: u32,
    pub recipient_id: i32,
    pub recipient_type: u32,
    pub parent_id: u32,
    pub pid_sender: u32,
    pub receptiontime: KerberosDateTime,
    pub life_time: u32,
    pub flags: u32,
    pub subject: String,
    pub sender: String,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct TextMessage {
    #[extends]
    pub msg: UserMessage,
    pub text_body: String,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct BinaryMessage {
    #[extends]
    pub msg: UserMessage,
    pub text_body: QBuffer,
}
