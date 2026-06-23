use std::env;
use std::io::{Cursor, Write};
use std::ops::Deref;
use std::sync::{Arc, atomic::AtomicU32};
use std::sync::{LazyLock, Weak};

use base64::{engine::general_purpose, Engine as _};
use bytemuck::{bytes_of, Pod, Zeroable};
use hmac::Mac;
use log::info;
use macros::rmc_struct;
use rnex_core::rmc::protocols::account_management::{
    AccountExtraInfo, AccountManagement, RawAccountManagement, RawAccountManagementInfo,
    RemoteAccountManagement,
};
use rnex_core::rmc::protocols::friends_wiiu::{
    FriendsWiiU, RawFriendsWiiU, RawFriendsWiiUInfo, RemoteFriendsWiiU,
};
use rnex_core::rmc::protocols::friends_3ds::{
    Friends3DS, RawFriends3DS, RawFriends3DSInfo, RemoteFriends3DS,
};
use rnex_core::rmc::protocols::nintendo_notification::{
    NintendoNotification, RawNintendoNotification, RawNintendoNotificationInfo,
    RemoteNintendoNotification,
};
use rnex_core::rmc::protocols::secure::{RawSecure, RawSecureInfo, RemoteSecure, Secure};
use rnex_core::{
    define_rmc_proto,
    kerberos::KerberosDateTime,
    nex::common::get_station_urls,
    prudp::{socket_addr::PRUDPSockAddr, station_url::StationUrl},
    rmc::{
        protocols::friends_wiiu::{
            BlacklistedPrincipal, Comment, FriendInfo, FriendRequest, NNAInfo, NintendoPresenceV2,
            PersistentNotification, PrincipalPreference, PrincipalRequestBlockSetting,
        },
        response::ErrorCode,
        structures::{any::Any, qresult::QResult},
    },
};
use sqlx::query;
use std::sync::atomic::Ordering::Relaxed;
use hex::decode;
use tokio::spawn;
use tokio::sync::RwLock;

use rnex_core::rmc::protocols::friends_wiiu::{GameKey, MiiV2, PrincipalBasicInfo};

use rnex_core::PID;

use rnex_core::rmc::protocols::account_management::NintendoCreateAccountData;
use rnex_core::rmc::protocols::nintendo_notification::NintendoNotificationEvent;
use rnex_core::rmc::structures::RmcSerialize;

use rnex_core::rmc::structures::data::Data;

use crate::executables::common::get_db;

use nex_account::derive_pid_hmac;
use nex_account::grpc::ActCreateInfo;
use nex_account::grpc::nex_account_service_client::NexAccountServiceClient;
use crate::rmc::protocols::friends_3ds::{FriendComment, FriendMii, FriendMiiList, FriendPersistentInfo, FriendPicture, FriendPresence, FriendRelationship, Mii, MiiList, MyProfile, NintendoPresence, PlayedGame};

define_rmc_proto!(
    proto FriendsUser{
        Secure,
        FriendsWiiU,
        Friends3DS
    }
);
define_rmc_proto!(
    proto FriendRemote{
        NintendoNotification
    }
);
define_rmc_proto!(
    proto FriendsGuest{
        Secure,
        AccountManagement
    }
);

static NEX_ACCOUNT_URL: LazyLock<String> =
    LazyLock::new(|| env::var("NEX_ACCOUNT_ENDPOINT").expect("NEX_ACCOUNT_ENDPOINT not set"));

pub struct UserData {
    info: NNAInfo,
    presence: NintendoPresenceV2,
}

#[repr(C, packed)]
#[derive(Pod, Zeroable, Copy, Clone, Debug)]
pub struct NascToken {
    pub pid: i32,
    pub time: [u8; 14],
    pub pwd_hash: [u8; 4],
}


#[rmc_struct(FriendsUser)]
pub struct FriendsUser {
    pub fm: Arc<FriendsManager>,
    pub addr: PRUDPSockAddr,
    pub pid: PID,
    pub data: RwLock<Option<UserData>>,
    pub current_friends: RwLock<Vec<PID>>,
    pub this: Weak<FriendsUser>,
    pub remote: RemoteFriendRemote,
}

#[rmc_struct(FriendsGuest)]
pub struct FriendsGuest {
    pub fm: Arc<FriendsManager>,
    pub addr: PRUDPSockAddr,
}

pub struct FriendsManager {
    pub cid_counter: AtomicU32,
    pub users: RwLock<Vec<Weak<FriendsUser>>>,
}

impl FriendsManager {
    pub fn next_cid(&self) -> u32 {
        self.cid_counter.fetch_add(1, Relaxed)
    }
}

// ALL of this is stubbed
impl Friends3DS for FriendsUser {
    async fn update_profile(&self, profile: MyProfile) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_mii(&self, profile: Mii) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_mii_list(&self, profile: MiiList) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_played_games(&self, profile: Vec<PlayedGame>) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_preference(&self, show_online_status: bool, show_current_title: bool, block_friend_requests: bool) -> Result<(), ErrorCode> {
        // stubbed
        Ok(())
    }

    async fn get_friend_mii(&self, friends: Vec<crate::rmc::protocols::friends_3ds::FriendInfo>) -> Result<Vec<FriendMii>, ErrorCode> {
        // sorry for the copying pretendo but i don't have a mii on hand rn
        let data: Vec<u8> = vec![
            0x03, 0x00, 0x00, 0x40, 0xE9, 0x55, 0xA2, 0x09,
            0xE7, 0xC7, 0x41, 0x82, 0xD9, 0x7D, 0x0B, 0x2D,
            0x03, 0xB3, 0xB8, 0x8D, 0x27, 0xD9, 0x00, 0x00,
            0x01, 0x40, 0x62, 0x00, 0x65, 0x00, 0x6C, 0x00,
            0x6C, 0x00, 0x61, 0x00, 0x00, 0x00, 0x45, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x40,
            0x12, 0x00, 0x81, 0x01, 0x04, 0x68, 0x43, 0x18,
            0x20, 0x34, 0x46, 0x14, 0x81, 0x12, 0x17, 0x68,
            0x0D, 0x00, 0x00, 0x29, 0x03, 0x52, 0x48, 0x50,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0x86,
        ];

        let dummymii = FriendMii {
            data: Data {},
            pid: 69,
            mii: Mii {
                data: Data {},
                name: "test".to_string(),
                profanity: false,
                char_set: 0,
                mii_data: data,
            },
            modified_at: Default::default(),
        };

        Ok(vec![dummymii])
    }

    async fn get_friend_mii_list(&self, friends: Vec<crate::rmc::protocols::friends_3ds::FriendInfo>) -> Result<Vec<FriendMiiList>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn is_active_game(&self, unk: Vec<u32>, game_key: crate::rmc::protocols::friends_3ds::GameKey) -> Result<Vec<u32>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_principal_id_by_local_friend_code(&self, unk1: u64, unk2: Vec<u64>) -> Result<Vec<FriendRelationship>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_relationships(&self, unk2: Vec<u32>) -> Result<Vec<FriendRelationship>, ErrorCode> {
        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 1,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn add_friend_by_pid(&self, unk: u64, pid: PID) -> Result<FriendRelationship, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_friend_by_lst_pid(&self, unk: u64, pid: Vec<PID>) -> Result<Vec<FriendRelationship>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_friend_by_local_code(&self, local_code: u64) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_friend_by_pid(&self, pid: PID) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_all_friends(&self) -> Result<Vec<FriendRelationship>, ErrorCode> {
        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 1,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn update_blacklist(&self) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn sync_friend(&self, unk1: u64, unk2: Vec<u32>, unk3: Vec<u64>) -> Result<Vec<FriendRelationship>, ErrorCode> {
        log::info!("params: {:?}, {:?}, {:?}", unk1, unk2, unk3);

        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 1,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn update_presence(&self, nintendo_presence: NintendoPresence, unk: bool) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_favorite_game_key(&self, game_key: rnex_core::rmc::protocols::friends_3ds::GameKey) -> Result<(), ErrorCode> {
        log::info!("favorite game key: {:?}", game_key);

        Ok(())
    }

    async fn update_comment(&self, comment: String) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_picture(&self, unk: u32, picture: Vec<u8>) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_presence(&self, unk: Vec<u32>) -> Result<Vec<FriendPresence>, ErrorCode> {
        let presence = FriendPresence {
            data: Data {},
            pid: 69,
            presence: NintendoPresence {
                data: Data {},
                changed_bit_flag: 0,
                game_key: rnex_core::rmc::protocols::friends_3ds::GameKey {
                    data: Data {},
                    title_id: 0x0005000010176900,
                    version: 0,
                },
                game_mode_desctiption: "".to_string(),
                join_availibility_flag: 0,
                mm_system_type: 0,
                join_game_id: 0,
                join_game_mode: 0,
                owner_pid: 0,
                join_group_id: 0,
                application_arg: vec![],
            },
        };

        Ok(vec![presence])
    }

    async fn get_friend_comment(&self, unk: Vec<crate::rmc::protocols::friends_3ds::FriendInfo>) -> Result<Vec<FriendComment>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_picture(&self, unk: Vec<u32>) -> Result<Vec<FriendPicture>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_persistent_info(&self, unk: Vec<u32>) -> Result<Vec<FriendPersistentInfo>, ErrorCode> {
        let dummypersistentinfo = FriendPersistentInfo {
            data: Data {},
            pid: 69,
            region: 0,
            country: 0,
            area: 0,
            language: 0,
            platform: 0,
            game_key: rnex_core::rmc::protocols::friends_3ds::GameKey {
                data: Data {},
                title_id: 0x0005000010176900,
                version: 0,
            },
            message: "yo whats up".to_string(),
            msg_updated_at: KerberosDateTime::now(),
            friended_at: KerberosDateTime::now(),
            last_online: KerberosDateTime::now(),
        };

        Ok(vec![dummypersistentinfo])
    }

    async fn send_invitation(&self, unk: Vec<u32>) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }
}

impl FriendsWiiU for FriendsUser {
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
    > {
        // let query = query!("select ", self.pid).fetch_all(get_db()).await;
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_friend(&self, friend: PID) -> Result<(FriendRequest, FriendInfo), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_friend_by_name(
        &self,
        name: String,
    ) -> Result<(FriendRequest, FriendInfo), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_friend(&self, friend: PID) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_friend_request(
        &self,
        friend: PID,
        unk1: u8,
        message: String,
        unk2: u8,
        unk3: String,
        game_key: GameKey,
        unk4: KerberosDateTime,
    ) -> Result<(FriendRequest, FriendInfo), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn cancel_friend_request(&self, id: u64) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn accept_friend_request(&self, id: u64) -> Result<FriendInfo, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn delete_friend_request(&self, id: u64) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn deny_friend_request(&self, id: u64) -> Result<BlacklistedPrincipal, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn mark_friend_requests_as_received(&self, ids: Vec<u64>) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_blacklist(
        &self,
        principal: BlacklistedPrincipal,
    ) -> Result<BlacklistedPrincipal, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_blacklist(&self, id: PID) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn update_presence(&self, presence: NintendoPresenceV2) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn update_mii(&self, presence: MiiV2) -> Result<KerberosDateTime, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn update_comment(&self, presence: Comment) -> Result<KerberosDateTime, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn update_preference(&self, preference: PrincipalPreference) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_basic_info(&self, pids: Vec<PID>) -> Result<Vec<PrincipalBasicInfo>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn delete_persistent_notification(
        &self,
        notifs: Vec<PersistentNotification>,
    ) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn check_setting_status(&self) -> Result<u8, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_request_block_settings(
        &self,
        unk: Vec<u32>,
    ) -> Result<Vec<PrincipalRequestBlockSetting>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }
}

type HMacMd5 = hmac::Hmac<md5::Md5>;

impl Secure for FriendsUser {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.fm.next_cid();
        let users = self.fm.users.read().await;
        if users.iter().filter(|u| u.upgrade().is_some()).count() >= 100 {
            return Err(ErrorCode::RendezVous_ConnectionFailure);
        }
        Ok((
            QResult::success(ErrorCode::Core_Unknown),
            cid,
            get_station_urls(&station_urls, self.addr, self.pid, cid).await?[0].clone(),
        ))
    }
    async fn register_ex(
        &self,
        station_urls: Vec<StationUrl>,
        _data: Any,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        info!("register");
        self.register(station_urls).await
    }
    async fn replace_url(&self, _target: StationUrl, _dest: StationUrl) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }
}

impl Secure for FriendsGuest {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.fm.next_cid();
        Ok((
            QResult::success(ErrorCode::Core_Unknown),
            cid,
            get_station_urls(&station_urls, self.addr, 100, cid).await?[0].clone(),
        ))
    }
    async fn register_ex(
        &self,
        station_urls: Vec<StationUrl>,
        _data: Any,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        info!("register");
        self.register(station_urls).await
    }
    async fn replace_url(&self, _target: StationUrl, _dest: StationUrl) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }
}


// this is probably a horrible way to do this. it's 1:27am. fuck off please i'll fix it later.
fn decode_token(encoded_str: &str) -> Result<NascToken, &'static str> {
    // i dont like the use of replace here as this will reallocate the entire string
    // im gonna keep it though as long as noone comes up with a good alternative for this
    // without complicating the code/making it far less readable/unnescesarily long
    let standard_b64 = encoded_str.replace('.', "+").replace('-', "/").replace('*', "=");

    let standard = general_purpose::STANDARD.decode(standard_b64).map_err(|_| "failed to convert")?;

    let bytes = general_purpose::STANDARD
        .decode(standard)
        .map_err(|_| "failed to decode Base64 string")?;

    if bytes.len() != std::mem::size_of::<NascToken>() {
        return Err("decoded byte length mismatch");
    }

    let token = bytemuck::from_bytes::<NascToken>(&bytes);

    Ok(*token)
}

impl AccountManagement for FriendsGuest {
    async fn nintendo_create_account(
        &self,
        principal_name: String,
        key: String,
        groups: u32,
        email: String,
        auth_data: Any,
    ) -> Result<(PID, String), ErrorCode> {
        println!("{}, {}, {}, {}", principal_name, key, groups, email);

        if let Ok(data) = auth_data.try_get_as::<NintendoCreateAccountData>() {
            let pid = data.nna_info.principal_basic_info.pid;
            info!("create account via standard data: {}", pid);

            let nexkey: [u8; 16] = hex::decode(key)
                .map_err(|_| ErrorCode::Authentication_InvalidParam)?
                .as_slice()
                .try_into()
                .map_err(|_| ErrorCode::Authentication_InvalidParam)?;

            let mac = derive_pid_hmac(data.nna_info.principal_basic_info.pid, &nexkey);

            let hex_str = hex::encode(mac);
            return Ok((pid, hex_str));
        }

        if let Ok(extra_info) = auth_data.try_get_as::<AccountExtraInfo>() {
            info!("create account via extra info");

            let decoded_token = decode_token(&*extra_info.nex_token).map_err(|e| {
                log::error!("failed to decode token: {}", e);
                log::info!("{:?}", extra_info.nex_token);
                ErrorCode::Authentication_InvalidParam
            })?;

            log::info!("decoded token: {:?}", decoded_token);

            let mut client = NexAccountServiceClient::connect(NEX_ACCOUNT_URL.as_str())
                .await
                .map_err(|e| {
                    eprintln!("error occurred: {:?}", e);
                    ErrorCode::Core_Unknown
                })?;

            let pid = decoded_token.pid;

            let new_account: ActCreateInfo = ActCreateInfo {
                principal_name,
                key: vec![],
                email,
                pid,
            };

            let nexkey = client
                .create_new_sequential_or_update_and_get_account(new_account)
                .await
                .map_err(|e| {ErrorCode::Core_Unknown})?
                .into_inner();

            if nexkey.key.len() != 16 {
                log::error!("nex key was not 16 bytes long");
                return Err(ErrorCode::Authentication_InvalidParam);
            }

            let nexkeyarray: [u8; 16] = nexkey.key.try_into()
                .expect("how...?");

            let mac = derive_pid_hmac(pid, &nexkeyarray);

            let hex_str = hex::encode(mac);

            return Ok((pid, hex_str));
        }

        Err(ErrorCode::Authentication_InvalidParam)
    }
}
