use macros::{RmcSerialize, method_id, rmc_proto};

use rnex_core::{kerberos::KerberosDateTime, rmc::response::ErrorCode};

use rnex_core::rmc::structures::data::Data;

use rnex_core::PID;

#[derive(RmcSerialize, Debug, Clone, Default)]
#[rmc_struct(0)]
pub struct MiiV2 {
    #[extends]
    pub data: Data,
    pub name: Vec<u8>,
    pub unk: u8,
    pub unk2: u8,
    pub mii_data: Vec<u8>,
    pub date_time: KerberosDateTime,
}

#[derive(RmcSerialize, Debug, Clone, Default)]
#[rmc_struct(0)]
pub struct PrincipalBasicInfo {
    #[extends]
    pub data: Data,
    pub pid: PID,
    pub nnid: String,
    pub mii: MiiV2,
    pub unk: u8,
}

#[derive(RmcSerialize, Debug, Clone, Default)]
#[rmc_struct(0)]
pub struct NNAInfo {
    #[extends]
    pub data: Data,
    pub principal_basic_info: PrincipalBasicInfo,
    pub unk: u8,
    pub unk2: u8,
}

#[derive(RmcSerialize, Clone, Copy, Debug, Default)]
#[rmc_struct(0)]
pub struct GameKey {
    #[extends]
    pub data: Data,
    pub tid: i64,
    pub version: i16,
}

#[derive(RmcSerialize, Clone, Debug, Default)]
#[rmc_struct(0)]
pub struct NintendoPresenceV2 {
    #[extends]
    pub data: Data,
    pub changed_flags: u32,
    pub is_online: bool,
    pub game_key: GameKey,
    pub unk: u8,
    pub message: String,
    pub unk2: u32,
    pub unk3: u8,
    pub game_server_id: u32,
    pub unk4: u32,
    pub pid: u32,
    pub gid: u32,
    pub app_data: Vec<u8>,
    pub unk5: u8,
    pub unk6: u8,
    pub unk7: u8,
}
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct PrincipalPreference {
    #[extends]
    pub data: Data,
    pub show_online: bool,
    pub show_playing_title: bool,
    pub block_friend_request: bool,
}

#[derive(RmcSerialize, Default, Debug)]
#[rmc_struct(0)]
pub struct Comment {
    #[extends]
    pub data: Data,
    pub unk: u8,
    pub message: String,
    pub last_changed: KerberosDateTime,
}

#[derive(RmcSerialize, Default, Debug)]
#[rmc_struct(0)]
pub struct FriendInfo {
    #[extends]
    pub data: Data,
    pub nna_info: NNAInfo,
    pub presence: NintendoPresenceV2,
    pub comment: Comment,
    pub became_friends: KerberosDateTime,
    pub last_online: KerberosDateTime,
    pub unk: u64,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct FriendRequestMessage {
    #[extends]
    pub data: Data,
    pub friend_request_id: i64,
    pub is_recieved: bool,
    pub unk: u8,
    pub message: String,
    pub unk2: u8,
    pub unk3: String,
    pub game_key: GameKey,
    pub unk4: KerberosDateTime,
    pub expires_on: KerberosDateTime,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct FriendRequest {
    #[extends]
    pub data: Data,
    pub basic_info: PrincipalBasicInfo,
    pub request_message: FriendRequestMessage,
    pub sent_on: KerberosDateTime,
}

#[derive(RmcSerialize, Debug, Default)]
#[rmc_struct(0)]
pub struct BlacklistedPrincipal {
    #[extends]
    pub data: Data,
    pub basic_info: PrincipalBasicInfo,
    pub game_key: GameKey,
    pub since: KerberosDateTime,
}
#[derive(RmcSerialize, Debug)]
#[rmc_struct(0)]
pub struct PersistentNotification {
    #[extends]
    pub data: Data,
    pub unk1: u64,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: String,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct PrincipalRequestBlockSetting {
    #[extends]
    pub data: Data,
    pub pid: PID,
    pub blocked: bool,
}

#[rmc_proto(102)]
pub trait FriendsWiiU {
    #[method_id(1)]
    async fn update_and_get_all_information(
        &self,
        info: NNAInfo,
        presence: NintendoPresenceV2,
        date_time: KerberosDateTime,
    ) -> Result<
        (
            PrincipalPreference,
            Comment,
            Vec<FriendInfo>,
            Vec<FriendRequest>,
            Vec<FriendRequest>,
            Vec<BlacklistedPrincipal>,
            bool,
            Vec<PersistentNotification>,
            bool,
        ),
        ErrorCode,
    >;
    #[method_id(2)]
    async fn add_friend(&self, friend: PID) -> Result<(FriendRequest, FriendInfo), ErrorCode>;
    #[method_id(3)]
    async fn add_friend_by_name(
        &self,
        name: String,
    ) -> Result<(FriendRequest, FriendInfo), ErrorCode>;
    #[method_id(4)]
    async fn remove_friend(&self, friend: PID) -> Result<(), ErrorCode>;
    #[method_id(5)]
    async fn add_friend_request(
        &self,
        friend: PID,
        unk1: u8,
        message: String,
        unk2: u8,
        unk3: String,
        game_key: GameKey,
        unk4: KerberosDateTime,
    ) -> Result<(FriendRequest, FriendInfo), ErrorCode>;
    #[method_id(6)]
    async fn cancel_friend_request(&self, id: u64) -> Result<(), ErrorCode>;
    #[method_id(7)]
    async fn accept_friend_request(&self, id: u64) -> Result<FriendInfo, ErrorCode>;
    #[method_id(8)]
    async fn delete_friend_request(&self, id: u64) -> Result<(), ErrorCode>;
    #[method_id(9)]
    async fn deny_friend_request(&self, id: u64) -> Result<BlacklistedPrincipal, ErrorCode>;
    #[method_id(10)]
    async fn mark_friend_requests_as_received(&self, ids: Vec<u64>) -> Result<(), ErrorCode>;
    #[method_id(11)]
    async fn add_blacklist(
        &self,
        principal: BlacklistedPrincipal,
    ) -> Result<BlacklistedPrincipal, ErrorCode>;
    #[method_id(12)]
    async fn remove_blacklist(&self, id: PID) -> Result<(), ErrorCode>;
    #[method_id(13)]
    async fn update_presence(&self, presence: NintendoPresenceV2) -> Result<(), ErrorCode>;
    #[method_id(14)]
    async fn update_mii(&self, presence: MiiV2) -> Result<KerberosDateTime, ErrorCode>;
    #[method_id(15)]
    async fn update_comment(&self, presence: Comment) -> Result<KerberosDateTime, ErrorCode>;
    #[method_id(16)]
    async fn update_preference(&self, preference: PrincipalPreference) -> Result<(), ErrorCode>;
    #[method_id(17)]
    async fn get_basic_info(&self, pids: Vec<PID>) -> Result<Vec<PrincipalBasicInfo>, ErrorCode>;
    #[method_id(18)]
    async fn delete_persistent_notification(
        &self,
        notifs: Vec<PersistentNotification>,
    ) -> Result<(), ErrorCode>;
    #[method_id(19)]
    async fn check_setting_status(&self) -> Result<u8, ErrorCode>;
    #[method_id(20)]
    async fn get_request_block_settings(
        &self,
        pids: Vec<PID>,
    ) -> Result<Vec<PrincipalRequestBlockSetting>, ErrorCode>;
}
