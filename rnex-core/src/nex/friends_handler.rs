use std::io::{Cursor, Write};
use std::ops::Deref;
use std::sync::Weak;
use std::sync::{Arc, atomic::AtomicU32};

use bytemuck::bytes_of;
use hmac::Mac;
use log::info;
use macros::rmc_struct;
use rnex_core::rmc::protocols::account_management::{
    AccountManagement, RawAccountManagement, RawAccountManagementInfo, RemoteAccountManagement,
};
use rnex_core::rmc::protocols::friends_wiiu::{
    FriendsWiiU, RawFriendsWiiU, RawFriendsWiiUInfo, RemoteFriendsWiiU,
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
use tokio::spawn;
use tokio::sync::RwLock;

use rnex_core::rmc::protocols::friends_wiiu::{GameKey, MiiV2, PrincipalBasicInfo};

use rnex_core::PID;

use rnex_core::rmc::protocols::account_management::NintendoCreateAccountData;
use rnex_core::rmc::protocols::nintendo_notification::NintendoNotificationEvent;
use rnex_core::rmc::structures::RmcSerialize;

use rnex_core::rmc::structures::data::Data;

use crate::executables::common::get_db;

define_rmc_proto!(
    proto FriendsUser{
        Secure,
        FriendsWiiU
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

pub struct UserData {
    info: NNAInfo,
    presence: NintendoPresenceV2,
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
        if auth_data.name == "NintendoCreateAccountData" {
            let Ok(data) =
                NintendoCreateAccountData::deserialize(&mut Cursor::new(&auth_data.data))
            else {
                return Err(ErrorCode::Authentication_InvalidParam);
            };

            let pid = data.nna_info.principal_basic_info.pid;
            info!("create account: {}", pid);

            let Ok(mut mac) = HMacMd5::new_from_slice(key.as_bytes()) else {
                return Err(ErrorCode::Authentication_InvalidParam);
            };

            mac.write_all(bytes_of(&pid))
                .expect("failed to write to hmac???");
            let mac = mac.finalize().into_bytes();

            let hex_str = hex::encode(mac);

            return Ok((pid, hex_str));
        }
        Err(ErrorCode::Core_NotImplemented)
    }
}
