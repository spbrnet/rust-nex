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
use rnex_core::rmc::protocols::friends::{Friends, RawFriends, RawFriendsInfo, RemoteFriends};
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
        protocols::friends::{
            BlacklistedPrincipal, Comment, FriendInfo, FriendRequest, NNAInfo, NintendoPresenceV2,
            PersistentNotification, PrincipalPreference,
        },
        response::ErrorCode,
        structures::{any::Any, qresult::QResult},
    },
};
use std::sync::atomic::Ordering::Relaxed;
use tokio::spawn;
use tokio::sync::RwLock;

use rnex_core::rmc::protocols::friends::{GameKey, MiiV2, PrincipalBasicInfo};

use rnex_core::PID;

use rnex_core::rmc::protocols::account_management::NintendoCreateAccountData;
use rnex_core::rmc::protocols::nintendo_notification::NintendoNotificationEvent;
use rnex_core::rmc::structures::RmcSerialize;

define_rmc_proto!(
    proto FriendsUser{
        Secure,
        Friends
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

pub fn friend_info_from_user(data: &UserData) -> FriendInfo {
    FriendInfo {
        nna_info: data.info.clone(),
        presence: data.presence.clone(),
        comment: Comment {
            unk: 0,
            message: "litterally everyone is friends here =w=".to_string(),
            last_changed: KerberosDateTime::now(),
        },
        became_friends: KerberosDateTime::now(),
        last_online: KerberosDateTime::now(),
        unk: 0,
    }
}

impl Friends for FriendsUser {
    async fn update_and_get_all_information(
        &self,
        info: NNAInfo,
        presence: NintendoPresenceV2,
        _date_time: KerberosDateTime,
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
        println!("updating own data");
        let mut data = self.data.write().await;
        *data = Some(UserData { info, presence });
        let self_fr_info = friend_info_from_user(data.as_ref().unwrap());
        let Ok(any_self_fr_info) = Any::new(&self_fr_info) else {
            return Err(ErrorCode::RendezVous_ControlScriptFailure);
        };
        drop(data);

        let mut fr_list =
            vec![FriendInfo {
                became_friends: KerberosDateTime::now(),
                comment: Comment {
                    last_changed: KerberosDateTime::now(),
                    message: "I'm just a dummy account :3".to_string(),
                    unk: 0,
                },
                last_online: KerberosDateTime::now(),
                nna_info: NNAInfo {
                    principal_basic_info: PrincipalBasicInfo {
                        pid: 101,
                        nnid: "dummy:3".to_string(),
                        mii: MiiV2{
                            date_time: KerberosDateTime::now(),
                            name: "TheDummy".to_string(),
                            mii_data: hex::decode("030000402bd7c32986a771f2dc6b35e31da15e37ff7c0000391e6f006f006d0069000000000000000000000000004040001065033568641e2013661a611821640f0000290052485000000000000000000000000000000000000000000000e838").unwrap(),
                            unk: 0,
                            unk2: 0,
                        },
                        unk: 0
                    },
                    unk: 0,
                    unk2: 0
                },
                presence: NintendoPresenceV2{
                    changed_flags: 0,
                    message: "".to_string(),
                    app_data: vec![],
                    game_key: GameKey{
                        tid: 0x00050002101ce400,
                        version: 0x0
                    },
                    game_server_id: 0,
                    is_online: true,
                    gid: 0,
                    pid: 101,
                    unk: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                    unk5: 0,
                    unk6: 0,
                    unk7: 0
                },
                unk: 0
            }];

        println!("acquiring user and current friends locks");
        let users = self.fm.users.read().await;
        println!("started summing users");
        for u in users.deref().iter().filter_map(|u| u.upgrade()) {
            let data = u.data.read().await;
            let Some(inner_data) = data.as_ref() else {
                continue;
            };
            fr_list.push(friend_info_from_user(&inner_data));
            drop(data);

            let mut curr_friends = self.current_friends.write().await;
            curr_friends.push(u.pid);
            drop(curr_friends);

            let mut fr = u.current_friends.write().await;
            if !fr.contains(&self.pid) {
                fr.push(self.pid);
                drop(fr);
                let data = any_self_fr_info.clone();
                let u = u.clone();
                let sender = self.pid;
                spawn(async move {
                    u.remote
                        .process_nintendo_notification_event_1(NintendoNotificationEvent {
                            event_type: 30,
                            sender,
                            data,
                        })
                        .await;
                });
            } else {
                drop(fr);
            }
        }
        println!("finished summing users");
        drop(users);

        println!("adding self to users");
        let mut users = self.fm.users.write().await;
        users.push(self.this.clone());
        drop(users);

        println!("done...");
        Ok((
            PrincipalPreference {
                block_friend_request: false,
                show_online: false,
                show_playing_title: false,
            },
            Comment {
                last_changed: KerberosDateTime::now(),
                message: "".to_string(),
                unk: 0,
            },
            fr_list,
            vec![],
            vec![],
            vec![],
            false,
            vec![],
            false,
        ))
    }

    async fn update_presence(&self, presence: NintendoPresenceV2) -> Result<(), ErrorCode> {
        let mut data = self.data.write().await;
        let Some(inner_data) = data.as_mut() else {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        };
        inner_data.presence = presence;
        let Ok(any_self_fr_info) = Any::new(&inner_data.presence) else {
            return Err(ErrorCode::RendezVous_ControlScriptFailure);
        };
        drop(data);

        let users = self.fm.users.read().await;
        for u in users.deref().iter().filter_map(|u| u.upgrade()) {
            u.remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 24,
                    sender: self.pid,
                    data: any_self_fr_info.clone(),
                })
                .await;
        }
        drop(users);

        Ok(())
    }

    async fn check_setting_status(&self) -> Result<u8, ErrorCode> {
        Ok(0xFF)
    }
}

type HMacMd5 = hmac::Hmac<md5::Md5>;

impl Secure for FriendsUser {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.fm.next_cid();
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
