use rnex_rmc::{
    RmcSerialize,
    data::Data,
    list::Buffer,
    method_id,
    response::ErrorCode,
    rmc_proto,
    util::{PID, date_time::DateTime},
};

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct MyProfile {
    #[extends]
    pub data: Data,
    pub region: u8,
    pub country: u8,
    pub area: u8,
    pub language: u8,
    pub platform: u8,
    pub local_friend_code_seed: u64,
    pub mac_address: String,
    pub serial_number: String,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct Mii {
    #[extends]
    pub data: Data,
    pub name: String,
    pub profanity: bool,
    pub char_set: u8, // 0 is JPN/USA/EUR, 1 is CHN, 2 is KOR and 3 is TWN
    pub mii_data: Vec<u8>,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct MiiList {
    #[extends]
    pub data: Data,
    pub unk1: String,
    pub unk2: bool,
    pub unk3: u8,
    pub mii_data: Vec<Buffer>,
}

#[derive(RmcSerialize, Debug)]
#[rmc_struct(0)]
pub struct GameKey {
    #[extends]
    pub data: Data,
    pub title_id: u64,
    pub version: u16,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct PlayedGame {
    #[extends]
    pub data: Data,
    pub game_key: GameKey,
    pub date_time: DateTime,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendInfo {
    pub pid: u32,
    pub unk2: DateTime,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendMii {
    #[extends]
    pub data: Data,
    pub pid: PID,
    pub mii: Mii,
    pub modified_at: DateTime,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendMiiList {
    #[extends]
    pub data: Data,
    pub unk1: u32,
    pub mii_list: MiiList,
    pub unk2: DateTime,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendRelationship {
    #[extends]
    pub data: Data,
    pub pid: u32,
    pub local_friend_code: u64,
    pub relationship_type: u8,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct NintendoPresence {
    #[extends]
    pub data: Data,
    pub changed_bit_flag: u32,
    pub game_key: GameKey,
    pub game_mode_desctiption: String,
    pub join_availibility_flag: u32,
    pub mm_system_type: u8,
    pub join_game_id: u32,
    pub join_game_mode: u32,
    pub owner_pid: PID,
    pub join_group_id: u32,
    pub application_arg: Buffer,
}
#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendPresence {
    #[extends]
    pub data: Data,
    pub pid: u32,
    pub presence: NintendoPresence,
}
#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendComment {
    #[extends]
    pub data: Data,
    pub pid: PID,
    pub comment: String,
    pub modified_at: DateTime,
}
#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendPicture {
    #[extends]
    pub data: Data,
    pub unk1: u32,
    pub pic_data: Vec<u32>,
    pub date_time: DateTime,
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
pub struct FriendPersistentInfo {
    #[extends]
    pub data: Data,
    pub pid: PID,
    pub region: u8,
    pub country: u8,
    pub area: u8,
    pub language: u8,
    pub platform: u8,
    pub game_key: GameKey,
    pub message: String,
    pub msg_updated_at: DateTime,
    pub friended_at: DateTime,
    pub last_online: DateTime,
}

#[rmc_proto(101)]
pub trait Friends3DS {
    #[method_id(1)]
    async fn update_profile(&self, profile: MyProfile) -> Result<(), ErrorCode>;
    #[method_id(2)]
    async fn update_mii(&self, profile: Mii) -> Result<(), ErrorCode>;
    #[method_id(3)]
    async fn update_mii_list(&self, profile: MiiList) -> Result<(), ErrorCode>;
    #[method_id(4)]
    async fn update_played_games(&self, profile: Vec<PlayedGame>) -> Result<(), ErrorCode>;
    #[method_id(5)]
    async fn update_preference(
        &self,
        show_online_status: bool,
        show_current_title: bool,
        block_friend_requests: bool,
    ) -> Result<(), ErrorCode>;

    #[method_id(6)]
    async fn get_friend_mii(&self, friends: Vec<FriendInfo>) -> Result<Vec<FriendMii>, ErrorCode>;
    #[method_id(7)]
    async fn get_friend_mii_list(
        &self,
        friends: Vec<FriendInfo>,
    ) -> Result<Vec<FriendMiiList>, ErrorCode>;
    #[method_id(8)]
    async fn is_active_game(&self, unk: Vec<u32>, game_key: GameKey)
    -> Result<Vec<u32>, ErrorCode>;
    #[method_id(9)]
    async fn get_principal_id_by_local_friend_code(
        &self,
        unk1: u64,
        unk2: Vec<u64>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode>;
    #[method_id(10)]
    async fn get_friend_relationships(
        &self,
        unk2: Vec<u32>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode>;
    #[method_id(11)]
    async fn add_friend_by_pid(&self, unk: u64, pid: PID) -> Result<FriendRelationship, ErrorCode>;
    #[method_id(12)]
    async fn add_friend_by_lst_pid(
        &self,
        unk: u64,
        pid: Vec<PID>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode>;
    #[method_id(13)]
    async fn remove_friend_by_local_code(&self, local_code: u64) -> Result<(), ErrorCode>;
    #[method_id(14)]
    async fn remove_friend_by_pid(&self, pid: PID) -> Result<(), ErrorCode>;
    #[method_id(15)]
    async fn get_all_friends(&self) -> Result<Vec<FriendRelationship>, ErrorCode>;
    #[method_id(16)]
    async fn update_blacklist(&self) -> Result<(), ErrorCode>;
    #[method_id(17)]
    async fn sync_friend(
        &self,
        unk1: u64,
        unk2: Vec<u32>,
        unk3: Vec<u64>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode>;
    #[method_id(18)]
    async fn update_presence(
        &self,
        nintendo_presence: NintendoPresence,
        unk: bool,
    ) -> Result<(), ErrorCode>;
    #[method_id(19)]
    async fn update_favorite_game_key(&self, game_key: GameKey) -> Result<(), ErrorCode>;
    #[method_id(20)]
    async fn update_comment(&self, comment: String) -> Result<(), ErrorCode>;
    #[method_id(21)]
    async fn update_picture(&self, unk: u32, picture: Vec<u8>) -> Result<(), ErrorCode>;
    #[method_id(22)]
    async fn get_friend_presence(&self, unk: Vec<u32>) -> Result<Vec<FriendPresence>, ErrorCode>;
    #[method_id(23)]
    async fn get_friend_comment(
        &self,
        unk: Vec<FriendInfo>,
    ) -> Result<Vec<FriendComment>, ErrorCode>;
    #[method_id(24)]
    async fn get_friend_picture(&self, unk: Vec<u32>) -> Result<Vec<FriendPicture>, ErrorCode>;
    #[method_id(25)]
    async fn get_friend_persistent_info(
        &self,
        unk: Vec<u32>,
    ) -> Result<Vec<FriendPersistentInfo>, ErrorCode>;
    #[method_id(26)]
    async fn send_invitation(&self, unk: Vec<u32>) -> Result<(), ErrorCode>;
}
