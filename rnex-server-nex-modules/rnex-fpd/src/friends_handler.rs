use std::{
    collections::HashMap,
    mem,
    sync::{Arc, Weak},
    time::Duration,
};

use bytemuck::{Pod, Zeroable};
use chrono::{TimeZone, Utc};
use nex_account::{derive_pid_hmac, grpc::ActCreateInfo, grpc_client};
use rnex_fpd_protos::{
    LocalFriendsGuest, LocalFriendsUser, RemoteFriendRemote,
    account_management::{AccountExtraInfo, AccountManagement, NintendoCreateAccountData},
    friends_3ds::{
        self, FriendComment, FriendMii, FriendMiiList, FriendPersistentInfo, FriendPicture,
        FriendPresence, FriendRelationship, Friends3DS, Mii, MiiList, MyProfile, NintendoPresence,
        PlayedGame,
    },
    friends_wiiu::{
        self, BlacklistedPrincipal, Comment, FriendRequest, FriendRequestMessage, FriendsWiiU,
        MiiV2, NNAInfo, NintendoPresenceV2, PersistentNotification, PrincipalBasicInfo,
        PrincipalPreference, PrincipalRequestBlockSetting,
    },
    nintendo_notification::{
        NintendoNotificationEvent, NintendoNotificationEventGeneral, RemoteNintendoNotification,
    },
};
use rnex_rmc::{any::Any, data::Data, qbuffer::QBuffer, response::ErrorCode, rmc_struct};
use rnex_server::PassthroughInitModule;
use rnex_util::{PID, date_time::DateTime};
use sqlx::query;
use tokio::{spawn, sync::RwLock, time::sleep};
use tracing::{error, info, warn};

use crate::FriendsManager;

#[repr(C, packed)]
#[derive(Pod, Zeroable, Copy, Clone, Debug)]
pub struct NascToken {
    pub pid: i32,
    pub time: [u8; 14],
    pub pwd_hash: [u8; 4],
}

#[derive(Debug)]
#[rmc_struct(FriendsUser)]
pub struct FriendsUser {
    pub fm: PassthroughInitModule<FriendsManager>,
    //pub addr: PRUDPSockAddr,
    pub pid: PID,
    pub friend_pids: RwLock<Vec<PID>>,
    pub maybe_remote_friend: RwLock<HashMap<PID, Weak<FriendsUser>>>,
    pub presence: RwLock<Option<NintendoPresenceV2>>,
    pub this: Weak<FriendsUser>,
    pub remote: RemoteFriendRemote,
}

#[derive(Debug)]
#[rmc_struct(FriendsGuest)]
pub struct FriendsGuest;

impl FriendsManager {
    async fn denies_friend_requests(&self, pid: PID) -> Result<bool, ErrorCode> {
        query!("select principal_preference_block_friend_requests from nintendo_network_accounts where pid = $1", pid)
            .fetch_one(&self.db)
            .await
            .map_err(|_| ErrorCode::FPD_InvalidAccount)
            .map(|v| v.principal_preference_block_friend_requests)
    }
}

// ALL of this is stubbed
impl Friends3DS for FriendsUser {
    async fn update_profile(&self, _profile: MyProfile) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_mii(&self, _profile: Mii) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_mii_list(&self, _profile: MiiList) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_played_games(&self, _profile: Vec<PlayedGame>) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_preference(
        &self,
        _show_online_status: bool,
        _show_current_title: bool,
        _block_friend_requests: bool,
    ) -> Result<(), ErrorCode> {
        // stubbed
        Ok(())
    }

    async fn get_friend_mii(
        &self,
        _friends: Vec<friends_3ds::FriendInfo>,
    ) -> Result<Vec<FriendMii>, ErrorCode> {
        // sorry for the copying pretendo but i don't have a mii on hand rn
        let data: Vec<u8> = vec![
            0x03, 0x00, 0x00, 0x40, 0xE9, 0x55, 0xA2, 0x09, 0xE7, 0xC7, 0x41, 0x82, 0xD9, 0x7D,
            0x0B, 0x2D, 0x03, 0xB3, 0xB8, 0x8D, 0x27, 0xD9, 0x00, 0x00, 0x01, 0x40, 0x62, 0x00,
            0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x61, 0x00, 0x00, 0x00, 0x45, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x40, 0x40, 0x12, 0x00, 0x81, 0x01, 0x04, 0x68, 0x43, 0x18,
            0x20, 0x34, 0x46, 0x14, 0x81, 0x12, 0x17, 0x68, 0x0D, 0x00, 0x00, 0x29, 0x03, 0x52,
            0x48, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0x86,
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

    async fn get_friend_mii_list(
        &self,
        _friends: Vec<friends_3ds::FriendInfo>,
    ) -> Result<Vec<FriendMiiList>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn is_active_game(
        &self,
        _unk: Vec<u32>,
        _game_key: friends_3ds::GameKey,
    ) -> Result<Vec<u32>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_principal_id_by_local_friend_code(
        &self,
        _unk1: u64,
        _unk2: Vec<u64>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_relationships(
        &self,
        _unk2: Vec<u32>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode> {
        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 3_268_487_429_723_707_977,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn add_friend_by_pid(
        &self,
        _unk: u64,
        _pid: PID,
    ) -> Result<FriendRelationship, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn add_friend_by_lst_pid(
        &self,
        _unk: u64,
        _pid: Vec<PID>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_friend_by_local_code(&self, _local_code: u64) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn remove_friend_by_pid(&self, _pid: PID) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_all_friends(&self) -> Result<Vec<FriendRelationship>, ErrorCode> {
        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 3_268_487_429_723_707_977,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn update_blacklist(&self) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn sync_friend(
        &self,
        unk1: u64,
        unk2: Vec<u32>,
        unk3: Vec<u64>,
    ) -> Result<Vec<FriendRelationship>, ErrorCode> {
        info!("params: {:?}, {:?}, {:?}", unk1, unk2, unk3);

        let dummy = FriendRelationship {
            data: Data {},
            pid: 69,
            local_friend_code: 3_268_487_429_723_707_977,
            relationship_type: 1,
        };

        Ok(vec![dummy])
    }

    async fn update_presence(
        &self,
        _nintendo_presence: NintendoPresence,
        _unk: bool,
    ) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_favorite_game_key(
        &self,
        game_key: friends_3ds::GameKey,
    ) -> Result<(), ErrorCode> {
        info!("favorite game key: {:?}", game_key);

        Ok(())
    }

    async fn update_comment(&self, _comment: String) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn update_picture(&self, _unk: u32, _picture: Vec<u8>) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_presence(&self, unk: Vec<u32>) -> Result<Vec<FriendPresence>, ErrorCode> {
        info!("pids: {:?}", unk);

        let presence = FriendPresence {
            data: Data {},
            pid: 69,
            presence: NintendoPresence {
                data: Data {},
                changed_bit_flag: 0xFFFF_FFFF,
                game_key: friends_3ds::GameKey {
                    data: Data {},
                    title_id: 1_125_899_907_457_280,
                    version: 2064,
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

    async fn get_friend_comment(
        &self,
        _unk: Vec<friends_3ds::FriendInfo>,
    ) -> Result<Vec<FriendComment>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_picture(&self, _unk: Vec<u32>) -> Result<Vec<FriendPicture>, ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }

    async fn get_friend_persistent_info(
        &self,
        _unk: Vec<u32>,
    ) -> Result<Vec<FriendPersistentInfo>, ErrorCode> {
        let dummypersistentinfo = FriendPersistentInfo {
            data: Data {},
            pid: 69,
            region: 0,
            country: 0,
            area: 0,
            language: 0,
            platform: 0,
            game_key: friends_3ds::GameKey {
                data: Data {},
                title_id: 1_125_899_907_457_280,
                version: 2064,
            },
            message: "yo whats up".to_string(),
            msg_updated_at: DateTime::now(),
            friended_at: DateTime::now(),
            last_online: DateTime::now(),
        };

        Ok(vec![dummypersistentinfo])
    }

    async fn send_invitation(&self, _unk: Vec<u32>) -> Result<(), ErrorCode> {
        Err(ErrorCode::Core_NotImplemented)
    }
}

fn smoosh_to_i16(val1: u8, val2: u8) -> i16 {
    bytemuck::cast(((val1 as u16) << 8) | val2 as u16)
}
fn unsmoosh_from_i16(val: i16) -> (u8, u8) {
    let val: u16 = bytemuck::cast(val);
    ((val & 0xFF00 >> 8) as u8, (val & 0xFF) as u8)
}
macro_rules! basic_principal_from_record {
    ($record:expr) => {{
        let (mii_unk1, mii_unk2) = unsmoosh_from_i16($record.mii_unks);
        PrincipalBasicInfo {
            data: Data {},
            pid: $record.pid,
            nnid: $record.nnid.clone(),
            mii: MiiV2 {
                data: Data {},
                date_time: DateTime(bytemuck::cast($record.mii_unk_datetime)),
                mii_data: $record.mii_ffl_data.clone(),
                name: QBuffer($record.mii_name.clone()),
                unk: mii_unk1,
                unk2: mii_unk2,
            },
            unk: (bytemuck::cast::<_, u16>($record.principal_info_unk) & 0xFF) as u8,
        }
    }};
}

macro_rules! nna_info_from_record {
    ($record:expr) => {{
        let (nna_unk1, nna_unk2) = unsmoosh_from_i16($record.nna_info_unks_2);
        NNAInfo {
            data: Data {},
            principal_basic_info: basic_principal_from_record!($record),
            unk: nna_unk1,
            unk2: nna_unk2,
        }
    }};
}

macro_rules! game_key_from_record {
    ($record:expr) => {
        friends_wiiu::GameKey {
            data: Data {},
            tid: $record.game_key_tid,
            version: $record.game_key_version,
        }
    };
}

macro_rules! friend_request_from_record {
    ($record:expr) => {{
        let (unk, unk2) = unsmoosh_from_i16($record.unks_1);
        FriendRequest {
            data: Data {},
            basic_info: basic_principal_from_record!($record),
            request_message: FriendRequestMessage {
                data: Data {},
                expires_on: DateTime::from_naive(
                    Utc.timestamp_opt(Utc::now().timestamp() + 2592000, 0)
                        .unwrap()
                        .naive_utc(),
                ),
                friend_request_id: $record.id,
                game_key: game_key_from_record!($record),
                is_recieved: $record.is_recieved,
                message: $record.message,
                unk,
                unk2,
                unk3: $record.unk_2,
                unk4: DateTime(bytemuck::cast($record.unk_3)),
            },
            sent_on: DateTime::from_naive($record.creation_time),
        }
    }};
}

impl FriendsWiiU for FriendsUser {
    async fn update_and_get_all_information(
        &self,
        info: NNAInfo,
        presence: NintendoPresenceV2,
        birthday: DateTime,
    ) -> Result<
        (
            PrincipalPreference,
            Comment,
            Vec<friends_wiiu::FriendInfo>,
            Vec<FriendRequest>,
            Vec<FriendRequest>,
            Vec<BlacklistedPrincipal>,
            bool,
            Vec<PersistentNotification>,
            bool,
        ),
        ErrorCode,
    > {
        if info.principal_basic_info.pid != self.pid {
            return Err(ErrorCode::FPD_InvalidArgument);
        }

        *self.presence.write().await = Some(presence.clone());

        let Ok(query) = query!("
            insert into nintendo_network_accounts (
                pid, nnid, mii_name, mii_unks, mii_ffl_data, mii_unk_datetime, principal_info_unk, nna_info_unks_2, birthday
            ) values (
                $1,  $2,   $3,       $4,       $5,           $6              , $7,                 $8,              $9
            )
            on conflict (pid)
                do update set
                    mii_name = excluded.mii_name,
                    mii_unks = excluded.mii_unks,
                    mii_ffl_data = excluded.mii_ffl_data,
                    mii_unk_datetime = excluded.mii_unk_datetime,
                    principal_info_unk = excluded.principal_info_unk,
                    nna_info_unks_2 = excluded.nna_info_unks_2,
                    birthday = excluded.birthday
            returning comment_unk, comment_message, comment_lastchanged, principal_preference_show_online, principal_preference_show_currently_playing_title, principal_preference_block_friend_requests
            ",
                self.pid,
                info.principal_basic_info.nnid,
                info.principal_basic_info.mii.name.0,
                smoosh_to_i16(info.principal_basic_info.mii.unk, info.principal_basic_info.mii.unk2),
                info.principal_basic_info.mii.mii_data,
                bytemuck::cast::<_, i64>(info.principal_basic_info.mii.date_time.0),
                info.principal_basic_info.unk as i16,
                smoosh_to_i16(info.unk, info.unk2),
                bytemuck::cast::<_, i64>(birthday)
            )
            .fetch_one(&self.fm.db)
            .await else {
                error!("psql failed(unable to update user)");
                return Err(ErrorCode::Core_SystemError)
            };

        let Ok(outgoing_friend_requests) = query!(
            "
            select
                *
            from friend_requests
            inner join nintendo_network_accounts on recipient = nintendo_network_accounts.pid
            where sender = $1
            ",
            self.pid
        )
        .fetch_all(&self.fm.db)
        .await
        .map(|v| {
            v.into_iter()
                .map(|v| friend_request_from_record!(v))
                .collect::<Vec<_>>()
        }) else {
            error!("error whilest getting friend requests");
            return Err(ErrorCode::Core_SystemError);
        };

        let Ok(incoming_friend_requests) = query!(
            "
            select
                *
            from friend_requests
            inner join nintendo_network_accounts on sender = nintendo_network_accounts.pid
            where recipient = $1
            ",
            self.pid
        )
        .fetch_all(&self.fm.db)
        .await
        .map(|v| {
            v.into_iter()
                .map(|v| friend_request_from_record!(v))
                .collect::<Vec<_>>()
        }) else {
            error!("error whilest getting friend requests");
            return Err(ErrorCode::Core_SystemError);
        };

        let Ok(friends_raw) = query!(
            "
            select
                nintendo_network_accounts.pid,
                since,
                nnid,
                comment_message,
                comment_unk,
                comment_lastchanged,
                nna_info_unks_2,
                principal_info_unk,
                mii_name,
                mii_unks,
                mii_ffl_data,
                mii_unk_datetime,
                last_online
            from friendships_of_pid($1) as friendships
            inner join nintendo_network_accounts on friendships.pid = nintendo_network_accounts.pid",
            self.pid
        )
        .fetch_all(&self.fm.db)
        .await
        else {
            error!("error whilest getting friends");
            return Err(ErrorCode::Core_SystemError);
        };

        let mut friends = Vec::with_capacity(friends_raw.len());

        for friend in friends_raw {
            let Some(friend_since) = friend.since else {
                error!("this should absolutely never happen(psql messed up somehow)");
                return Err(ErrorCode::Core_SystemError);
            };

            let mut presence = NintendoPresenceV2::default();

            let mut current_friends = self.friend_pids.write().await;
            current_friends.push(friend.pid);
            drop(current_friends);

            let users = self.fm.users.read().await;
            let user = users.get(&friend.pid).cloned();
            drop(users);
            if let Some(user) = user
                && let Some(user) = user.upgrade()
            {
                let online_presence = user.presence.read().await;
                if let Some(online_presence) = online_presence.as_ref() {
                    presence = online_presence.clone();
                } else {
                    error!(
                        "internal server error, user is somehow online and in the friends manager yet has not set their presence yet..."
                    );
                }
                drop(online_presence);
                let mut other_friend_remotes = user.maybe_remote_friend.write().await;
                other_friend_remotes.insert(self.pid, self.this.clone());
                drop(other_friend_remotes);
                let mut remo_friends = self.maybe_remote_friend.write().await;
                remo_friends.insert(friend.pid, Arc::downgrade(&user));
                drop(remo_friends);
            }

            friends.push(friends_wiiu::FriendInfo {
                data: Data {},
                nna_info: nna_info_from_record!(friend),
                presence,
                comment: Comment {
                    data: Data {},
                    unk: (bytemuck::cast::<_, u16>(friend.comment_unk) & 0xFF) as u8,
                    message: friend.comment_message,
                    last_changed: DateTime::from_naive(friend.comment_lastchanged),
                },
                became_friends: DateTime::from_naive(friend_since),
                last_online: DateTime::from_naive(friend.last_online),
                unk: 0,
            });
        }

        let Ok(denylist) = query!(
            "
            select
                *
            from denylist
            inner join nintendo_network_accounts on other = nintendo_network_accounts.pid
            where initiator = $1
            ",
            self.pid
        )
        .fetch_all(&self.fm.db)
        .await
        .map(|v| {
            v.into_iter()
                .map(|v| BlacklistedPrincipal {
                    basic_info: basic_principal_from_record!(v),
                    since: DateTime::from_naive(v.since),
                    ..Default::default()
                })
                .collect::<Vec<_>>()
        }) else {
            error!("error whilest getting friend requests");
            return Err(ErrorCode::Core_SystemError);
        };

        self.fm
            .users
            .write()
            .await
            .insert(self.pid, self.this.clone());

        <Self as FriendsWiiU>::update_presence(self, presence).await?;
        dbg!(Ok((
            PrincipalPreference {
                data: Data {},
                block_friend_request: query.principal_preference_block_friend_requests,
                show_online: query.principal_preference_show_online,
                show_playing_title: query.principal_preference_show_currently_playing_title,
            },
            Comment {
                data: Data {},
                last_changed: DateTime::from_naive(query.comment_lastchanged),
                message: query.comment_message,
                unk: (bytemuck::cast::<_, u16>(query.comment_unk) & 0xFF) as u8,
            },
            friends,
            outgoing_friend_requests,
            incoming_friend_requests,
            // todo: blacklisted principals
            denylist,
            false,
            // todo: persistent notifications
            vec![],
            false,
        )))
    }

    async fn add_friend(
        &self,
        friend: PID,
    ) -> Result<(FriendRequest, friends_wiiu::FriendInfo), ErrorCode> {
        self.add_friend_request(
            friend,
            0,
            "".into(),
            0,
            "".into(),
            friends_wiiu::GameKey::default(),
            DateTime::now(),
        )
        .await
    }

    async fn add_friend_by_name(
        &self,
        name: String,
    ) -> Result<(FriendRequest, friends_wiiu::FriendInfo), ErrorCode> {
        let Ok(pid) = query!(
            "select pid from nintendo_network_accounts where nnid = $1",
            name
        )
        .fetch_optional(&self.fm.db)
        .await
        else {
            error!("db error when trying to look up nnid");
            return Err(ErrorCode::Core_Exception);
        };

        match pid {
            Some(pid) => self.add_friend(pid.pid).await,
            None => Err(ErrorCode::FPD_InvalidAccount),
        }
    }

    async fn remove_friend(&self, friend: PID) -> Result<(), ErrorCode> {
        let Ok(_) = query!(
            "delete from friendships where (pid_a = $1 AND pid_b = $2) OR (pid_a = $2 AND pid_b = $1)",
            self.pid,
            friend
        )
        .execute(&self.fm.db)
        .await
        else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let mut friends = self.friend_pids.write().await;
        friends.retain(|v| *v != friend);
        drop(friends);
        let mut friends = self.maybe_remote_friend.write().await;
        friends.remove(&friend);
        drop(friends);

        let users = self.fm.users.read().await;
        if let Some(user) = users.get(&friend).and_then(|v| v.upgrade()) {
            drop(users);
            let mut friends = user.friend_pids.write().await;
            friends.retain(|v| *v != self.pid);
            drop(friends);
            let mut friends = user.maybe_remote_friend.write().await;
            friends.remove(&self.pid);
            drop(friends);

            user.remote
                .process_nintendo_notification_event_1(NintendoNotificationEvent {
                    event_type: 26,
                    sender: self.pid,
                    data: Any::new(&NintendoNotificationEventGeneral {
                        param1: bytemuck::cast(self.pid),
                        ..Default::default()
                    })
                    .expect("type error"),
                })
                .await;
        }

        let mut friends = self.friend_pids.write().await;
        friends.retain(|v| *v != friend);
        drop(friends);
        let mut friends = self.maybe_remote_friend.write().await;
        friends.remove(&friend);
        drop(friends);

        Ok(())
    }

    async fn add_friend_request(
        &self,
        friend: PID,
        _unk1: u8,
        message: String,
        _unk2: u8,
        unk3: String,
        game_key: friends_wiiu::GameKey,
        unk4: DateTime,
    ) -> Result<(FriendRequest, friends_wiiu::FriendInfo), ErrorCode> {
        let unk1 = 0;
        let unk2 = 1;

        if self.fm.denies_friend_requests(friend).await? {
            return Err(ErrorCode::FPD_FriendRequestNotAllowed);
        }

        if friend == 99 {
            query!(
                "insert into friendships (pid_a, pid_b) values(99, $1)",
                self.pid
            )
            .execute(&self.fm.db)
            .await
            .ok();

            let Ok(v) = query!("select * from nintendo_network_accounts where pid = 99")
                .fetch_one(&self.fm.db)
                .await
            else {
                return Err(ErrorCode::Core_Exception);
            };
            let Some(this) = self.this.upgrade() else {
                return Err(ErrorCode::Core_Exception);
            };

            let pid = self.pid;

            spawn(async move {
                sleep(Duration::from_secs(5)).await;

                let mut friends = this.friend_pids.write().await;
                friends.push(99);
                drop(friends);

                let Ok(r) = query!("select * from nintendo_network_accounts where pid = 99")
                    .fetch_one(&this.fm.db)
                    .await
                else {
                    return;
                };
                let data = Any::new(&friends_wiiu::FriendInfo {
                    data: Data {},
                    nna_info: nna_info_from_record!(r),
                    became_friends: DateTime::now(),
                    comment: Comment {
                        data: Data {},
                        last_changed: DateTime::from_naive(r.comment_lastchanged),
                        message: r.comment_message,
                        unk: (bytemuck::cast::<_, u16>(r.comment_unk) & 0xFF) as u8,
                    },
                    last_online: DateTime::now(),
                    presence: NintendoPresenceV2::default(),
                    unk: 0,
                })
                .expect("type error");
                this.remote
                    .process_nintendo_notification_event_1(NintendoNotificationEvent {
                        event_type: 30,
                        sender: pid,
                        data,
                    })
                    .await;
            });

            return Ok((
                FriendRequest {
                    basic_info: basic_principal_from_record!(v),
                    request_message: FriendRequestMessage {
                        data: Data {},
                        friend_request_id: i64::MAX,
                        is_recieved: true,
                        unk: 0,
                        message,
                        unk2: 1,
                        unk3: "Dummy".into(),
                        game_key,
                        unk4: DateTime::now(),
                        expires_on: DateTime::now(),
                    },
                    data: Data {},
                    sent_on: DateTime::now(),
                },
                friends_wiiu::FriendInfo {
                    became_friends: DateTime::now(),
                    comment: Comment {
                        data: Data {},
                        unk: bytemuck::cast::<_, u16>(v.comment_unk & 0xFF) as u8,
                        message: v.comment_message,
                        last_changed: DateTime::from_naive(v.comment_lastchanged),
                    },
                    last_online: DateTime::now(),
                    nna_info: nna_info_from_record!(v),
                    ..Default::default()
                },
            ));
        }

        let Ok(q) = query!(
            "select * from denylist where initiator = $1 and other = $2",
            friend,
            self.pid
        )
        .fetch_optional(&self.fm.db)
        .await
        else {
            return Err(ErrorCode::Authentication_AccountLibraryError);
        };

        if q.is_some() {
            return Err(ErrorCode::FPD_FriendRequestBlocked);
        }

        // check for too many friend requests both ways
        let Ok(query) = query!(
            "select count(recipient) from friend_requests where sender = $1",
            self.pid
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            error!("friend request count check failed to execute on database");
            return Err(ErrorCode::Core_Exception);
        };
        if query.count.is_none_or(|v| v >= 100) {
            return Err(ErrorCode::FPD_RequestLimitExceed);
        }
        let Ok(query) = query!(
            "select count(recipient) from friend_requests where recipient = $1",
            friend
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            error!("friend request count check failed to execute on database");
            return Err(ErrorCode::Core_Exception);
        };
        if query.count.is_none_or(|v| v >= 100) {
            return Err(ErrorCode::FPD_RequestLimitExceed);
        }

        let unks_1 = smoosh_to_i16(unk1, unk2);
        let query = query!(
            "with fr_base as (insert into friend_requests
                (sender, recipient, message, unks_1, unk_2, unk_3, game_key_tid, game_key_version)
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning *)
            select *
                from fr_base
                inner join nintendo_network_accounts
                on recipient=pid
            ",
            //inner join on recipient = pid
            self.pid,
            friend,
            message,
            unks_1,
            unk3,
            bytemuck::cast::<_, i64>(unk4.0),
            game_key.tid,
            game_key.version
        )
        .fetch_one(&self.fm.db)
        .await;

        let query = match query {
            Ok(q) => q,
            Err(e) => {
                if let Some(e) = e.into_database_error() {
                    match e.kind() {
                        sqlx::error::ErrorKind::UniqueViolation => {
                            return Err(ErrorCode::FPD_AddFriendProhibited);
                        }
                        sqlx::error::ErrorKind::ForeignKeyViolation => {
                            return Err(ErrorCode::FPD_NotNetworkAccount); // incorrect error code, this is returned only by the system if the current user does not have an NNID
                        }
                        sqlx::error::ErrorKind::CheckViolation => {
                            return Err(ErrorCode::FPD_FriendRequestNotAllowed);
                        }
                        _ => {
                            error!("unknown db error occurred");
                            return Err(ErrorCode::Core_Exception);
                        }
                    }
                }
                error!("unknown db error occurred");
                return Err(ErrorCode::Core_Exception);
            }
        };

        let fr = friend_request_from_record!(query);

        let users = self.fm.users.read().await;
        if let Some(user) = users.get(&friend).and_then(|v| v.upgrade()) {
            let mut fr = fr.clone();

            let Ok(query) = query!(
                "select * from nintendo_network_accounts where pid = $1",
                self.pid
            )
            .fetch_one(&self.fm.db)
            .await
            else {
                warn!("failed to acquire account info after adding friend");
                return Err(ErrorCode::FPD_InvalidMessageID);
            };
            fr.basic_info = basic_principal_from_record!(query);

            user.remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 27,
                    sender: self.pid,
                    data: Any::new(&fr).expect("type check failed"),
                })
                .await;
        }

        dbg!(Ok((
            fr,
            friends_wiiu::FriendInfo {
                presence: NintendoPresenceV2 {
                    game_key,
                    app_data: vec![0x00],
                    ..Default::default()
                },
                ..Default::default()
            },
        )))
    }

    async fn cancel_friend_request(&self, id: u64) -> Result<(), ErrorCode> {
        let Ok(query) = query!(
            "delete from friend_requests where id = $1 and sender = $2 returning recipient",
            bytemuck::cast::<_, i64>(id),
            self.pid
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let users = self.fm.users.read().await;
        if let Some(user) = users.get(&query.recipient).and_then(|v| v.upgrade()) {
            drop(users);

            user.remote
                .process_nintendo_notification_event_1(NintendoNotificationEvent {
                    event_type: 26,
                    sender: self.pid,
                    data: Any::new(&NintendoNotificationEventGeneral {
                        param1: bytemuck::cast(self.pid),
                        ..Default::default()
                    })
                    .expect("type error"),
                })
                .await;
        }

        Ok(())
    }

    async fn accept_friend_request(&self, id: u64) -> Result<friends_wiiu::FriendInfo, ErrorCode> {
        let Ok(mut tx) = self.fm.db.begin().await else {
            return Err(ErrorCode::Core_Exception);
        };
        let Ok(query) = query!(
            "delete from friend_requests where id = $1 and recipient = $2 returning recipient, sender",
            bytemuck::cast::<_, i64>(id),
            self.pid
        )
        .fetch_one(&mut *tx)
        .await
        else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let sender = query.sender;

        match query!(
            "insert into friendships (pid_a, pid_b) values ($1, $2)",
            query.recipient,
            query.sender
        )
        .execute(&mut *tx)
        .await
        {
            Ok(_) => {}
            Err(_) => return Err(ErrorCode::FPD_FriendAlreadyAdded),
        };

        let Ok(query) = query!(
            "select * from nintendo_network_accounts where pid = $1",
            query.sender
        )
        .fetch_one(&mut *tx)
        .await
        else {
            warn!("failed to acquire account info after adding friend");
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let mut presence = NintendoPresenceV2::default();

        let users = self.fm.users.read().await;
        let user = users.get(&sender).cloned();
        drop(users);
        if let Some(user) = user
            && let Some(user) = user.upgrade()
        {
            let mut friends = user.friend_pids.write().await;
            friends.push(self.pid);
            drop(friends);
            let mut friends = user.maybe_remote_friend.write().await;
            friends.insert(self.pid, self.this.clone());
            drop(friends);

            let Ok(r) = query!(
                "select * from nintendo_network_accounts where pid = $1",
                self.pid
            )
            .fetch_one(&mut *tx)
            .await
            else {
                error!("internal server error whilest getting nna info");
                return Err(ErrorCode::Core_Exception);
            };
            let data = Any::new(&friends_wiiu::FriendInfo {
                data: Data {},
                nna_info: nna_info_from_record!(r),
                became_friends: DateTime::now(),
                comment: Comment {
                    data: Data {},
                    last_changed: DateTime::from_naive(r.comment_lastchanged),
                    message: r.comment_message,
                    unk: (bytemuck::cast::<_, u16>(r.comment_unk) & 0xFF) as u8,
                },
                last_online: DateTime::now(),
                presence: self.presence.read().await.clone().unwrap_or_default(),
                unk: 0,
            })
            .expect("type error");
            user.remote
                .process_nintendo_notification_event_1(NintendoNotificationEvent {
                    event_type: 30,
                    sender: self.pid,
                    data,
                })
                .await;
            let online_presence = user.presence.read().await;
            if let Some(online_presence) = online_presence.as_ref() {
                presence = online_presence.clone();
            } else {
                error!(
                    "internal server error, user is somehow online and in the friends manager yet has not set their presence yet..."
                );
            }
            drop(online_presence);
        }

        tx.commit().await.ok();

        Ok(friends_wiiu::FriendInfo {
            data: Data {},
            nna_info: nna_info_from_record!(query),
            presence,
            comment: Comment {
                data: Data {},
                last_changed: DateTime::from_naive(query.comment_lastchanged),
                message: query.comment_message,
                unk: (bytemuck::cast::<_, u16>(query.comment_unk) & 0xFF) as u8,
            },
            became_friends: DateTime::now(),
            last_online: DateTime::from_naive(query.last_online),
            unk: 0,
        })
    }

    async fn delete_friend_request(&self, id: u64) -> Result<(), ErrorCode> {
        let Ok(query) = query!(
            "delete from friend_requests where id = $1 returning sender, recipient",
            bytemuck::cast::<_, i64>(id),
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let other = if query.recipient == self.pid {
            query.sender
        } else if query.sender == self.pid {
            query.recipient
        } else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let users = self.fm.users.read().await;
        if let Some(user) = users.get(&other).and_then(|v| v.upgrade()) {
            drop(users);

            user.remote
                .process_nintendo_notification_event_1(NintendoNotificationEvent {
                    event_type: 26,
                    sender: self.pid,
                    data: Any::new(&NintendoNotificationEventGeneral {
                        param1: bytemuck::cast(self.pid),
                        ..Default::default()
                    })
                    .expect("type error"),
                })
                .await;
        }

        Ok(())
    }

    async fn deny_friend_request(&self, id: u64) -> Result<BlacklistedPrincipal, ErrorCode> {
        let Ok(query) = query!(
            "delete from friend_requests where id = $1 and recipient = $2 returning sender, recipient",
            bytemuck::cast::<_, i64>(id),
            self.pid
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let other = if query.recipient == self.pid {
            query.sender
        } else if query.sender == self.pid {
            query.recipient
        } else {
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        if let Err(e) = query!(
            "insert into denylist(initiator, other) VALUES ($1, $2)",
            self.pid,
            query.sender
        )
        .execute(&self.fm.db)
        .await
        {
            error!("friend request denial db error: {}", e);
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let users = self.fm.users.read().await;
        if let Some(user) = users.get(&other).and_then(|v| v.upgrade()) {
            drop(users);

            user.remote
                .process_nintendo_notification_event_1(NintendoNotificationEvent {
                    event_type: 26,
                    sender: self.pid,
                    data: Any::new(&NintendoNotificationEventGeneral {
                        param1: bytemuck::cast(self.pid),
                        param2: id,
                        ..Default::default()
                    })
                    .expect("type error"),
                })
                .await;
        }

        let Ok(user) = query!(
            "select * from nintendo_network_accounts where pid = $1",
            query.sender
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            error!("attempt to get invalid user which is in friend request");
            return Err(ErrorCode::FPD_InvalidArgument);
        };

        Ok(BlacklistedPrincipal {
            data: Data {},
            basic_info: basic_principal_from_record!(user),
            game_key: friends_wiiu::GameKey::default(),
            since: DateTime::now(),
        })
    }

    async fn mark_friend_requests_as_received(&self, ids: Vec<u64>) -> Result<(), ErrorCode> {
        for id in ids {
            query!(
                "update friend_requests set is_recieved = true where id = $1",
                bytemuck::cast::<_, i64>(id)
            )
            .execute(&self.fm.db)
            .await
            .ok();
        }
        Ok(())
    }

    async fn add_blacklist(
        &self,
        principal: BlacklistedPrincipal,
    ) -> Result<BlacklistedPrincipal, ErrorCode> {
        if let Err(e) = query!(
            "insert into denylist(initiator, other) VALUES ($1, $2)",
            self.pid,
            principal.basic_info.pid
        )
        .execute(&self.fm.db)
        .await
        {
            error!("failed to block user: {}", e);
            return Err(ErrorCode::FPD_InvalidMessageID);
        };

        let Ok(user) = query!(
            "select * from nintendo_network_accounts where pid = $1",
            principal.basic_info.pid
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            error!("attempt to get invalid user which is in friend request");
            return Err(ErrorCode::FPD_InvalidArgument);
        };

        Ok(BlacklistedPrincipal {
            data: Data {},
            basic_info: basic_principal_from_record!(user),
            game_key: friends_wiiu::GameKey::default(),
            since: DateTime::now(),
        })
    }

    async fn remove_blacklist(&self, id: PID) -> Result<(), ErrorCode> {
        if let Err(e) = query!(
            "delete from denylist where initiator = $1 and other = $2",
            self.pid,
            id
        )
        .execute(&self.fm.db)
        .await
        {
            error!("error removing blacklist: {}", e);
            return Err(ErrorCode::FPD_InvalidMessageID);
        };
        Ok(())
    }

    async fn update_presence(&self, mut presence: NintendoPresenceV2) -> Result<(), ErrorCode> {
        if !query!("select principal_preference_show_currently_playing_title from nintendo_network_accounts where pid = $1", self.pid).fetch_one(&self.fm.db).await.map_err(|_| ErrorCode::FPD_InvalidAccount)?.principal_preference_show_currently_playing_title{
            presence.game_server_id = 0;
            presence.game_key = friends_wiiu::GameKey::default();
            presence.app_data = vec![];
        }
        if !query!(
            "select principal_preference_show_online from nintendo_network_accounts where pid = $1",
            self.pid
        )
        .fetch_one(&self.fm.db)
        .await
        .map_err(|_| ErrorCode::FPD_InvalidAccount)?
        .principal_preference_show_online
        {
            presence.is_online = false;
        }

        presence.is_online = true;
        let data = Any::new(&presence).expect("type error");
        let mut user_presence = self.presence.write().await;
        *user_presence = Some(presence);
        drop(user_presence);

        let friends = self.maybe_remote_friend.read().await;
        for friend in friends.iter().filter_map(|f| f.1.upgrade()) {
            friend
                .remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 24,
                    sender: self.pid,
                    data: data.clone(),
                })
                .await;
        }
        Ok(())
    }

    async fn update_mii(&self, mii: MiiV2) -> Result<DateTime, ErrorCode> {
        if let Err(e) = query!(
            "
            update nintendo_network_accounts
            set mii_name = $1, mii_unks = $2, mii_ffl_data = $3, mii_unk_datetime = $4
            where pid = $5",
            mii.name.0,
            smoosh_to_i16(mii.unk, mii.unk2),
            mii.mii_data,
            bytemuck::cast::<_, i64>(mii.date_time.0),
            self.pid
        )
        .execute(&self.fm.db)
        .await
        {
            error!("internal server error whilest updating mii: {}", e);
            return Err(ErrorCode::Core_Exception);
        }

        let Ok(r) = query!(
            "select * from nintendo_network_accounts where pid = $1",
            self.pid
        )
        .fetch_one(&self.fm.db)
        .await
        else {
            error!("internal server error whilest getting nna info");
            return Err(ErrorCode::Core_Exception);
        };

        let nna_info = nna_info_from_record!(r);
        let data = Any::new(&nna_info).expect("type error");

        let friends = self.maybe_remote_friend.read().await;
        for friend in friends.iter().filter_map(|f| f.1.upgrade()) {
            friend
                .remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 21,
                    sender: self.pid,
                    data: data.clone(),
                })
                .await;
        }

        Ok(DateTime::now())
    }

    async fn update_comment(&self, comment: Comment) -> Result<DateTime, ErrorCode> {
        if let Err(e) = query!(
            "
            update nintendo_network_accounts
            set comment_unk = $1, comment_message = $2, comment_lastchanged = now()
            where pid = $3",
            comment.unk as i16,
            comment.message.clone(),
            self.pid
        )
        .execute(&self.fm.db)
        .await
        {
            error!("internal server error whilest updating mii: {}", e);
            return Err(ErrorCode::Core_Exception);
        }

        let data = Any::new(&NintendoNotificationEventGeneral {
            param1: bytemuck::cast(self.pid),
            str_param: comment.message,
            ..Default::default()
        })
        .expect("type error");
        let friends = self.maybe_remote_friend.read().await;
        for friend in friends.iter().filter_map(|f| f.1.upgrade()) {
            friend
                .remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 21,
                    sender: self.pid,
                    data: data.clone(),
                })
                .await;
        }

        Ok(DateTime::now())
    }

    async fn update_preference(&self, preference: PrincipalPreference) -> Result<(), ErrorCode> {
        let data = Any::new(&preference).expect("type error");

        if let Err(e) = query!(
            "
            update nintendo_network_accounts
            set
            principal_preference_show_online = $1, principal_preference_show_currently_playing_title = $2, principal_preference_block_friend_requests = $3
            where pid = $4",
            preference.show_online,
            preference.show_playing_title,
            preference.block_friend_request,
            self.pid
        )
        .execute(&self.fm.db)
        .await
        {
            error!("internal server error whilest updating mii: {e}");
            return Err(ErrorCode::Core_Exception);
        }

        let mut presence = self.presence.write().await;
        let Some(presence) = presence.as_mut() else {
            return Err(ErrorCode::FPD_InvalidState);
        };

        if !preference.show_playing_title {
            presence.game_server_id = 0;
            presence.game_key = friends_wiiu::GameKey::default();
            presence.app_data = vec![];
        }
        query!(
            "update nintendo_network_accounts set last_online = now() where pid = $1",
            self.pid
        )
        .execute(&self.fm.db)
        .await
        .ok();
        let friends = self.maybe_remote_friend.read().await;
        for friend in friends.iter().filter_map(|f| f.1.upgrade()) {
            friend
                .remote
                .process_nintendo_notification_event_2(NintendoNotificationEvent {
                    event_type: 23,
                    sender: self.pid,
                    data: data.clone(),
                })
                .await;
            if preference.show_online {
                friend
                    .remote
                    .process_nintendo_notification_event_2(NintendoNotificationEvent {
                        event_type: 24,
                        sender: self.pid,
                        data: Any::new(presence).expect("type error"),
                    })
                    .await;
            } else {
                friend
                    .remote
                    .process_nintendo_notification_event_2(NintendoNotificationEvent {
                        event_type: 10,
                        sender: self.pid,
                        data: Any::new(&NintendoNotificationEventGeneral {
                            param3: DateTime::now().0,
                            ..Default::default()
                        })
                        .expect("type error"),
                    })
                    .await;
            }
        }

        Ok(())
    }

    async fn get_basic_info(&self, pids: Vec<PID>) -> Result<Vec<PrincipalBasicInfo>, ErrorCode> {
        let mut principal_infos = Vec::with_capacity(pids.len());

        for pid in pids {
            info!("looking up pid: {pid}");
            let Ok(user) = query!(
                "select * from nintendo_network_accounts where pid = $1",
                pid
            )
            .fetch_one(&self.fm.db)
            .await
            else {
                return Err(ErrorCode::FPD_NotFriend);
            };

            principal_infos.push(basic_principal_from_record!(user));
        }

        Ok(principal_infos)
    }

    async fn delete_persistent_notification(
        &self,
        _notifs: Vec<PersistentNotification>,
    ) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn check_setting_status(&self) -> Result<u8, ErrorCode> {
        Ok(0)
    }

    async fn get_request_block_settings(
        &self,
        pids: Vec<PID>,
    ) -> Result<Vec<PrincipalRequestBlockSetting>, ErrorCode> {
        let mut blocked: Vec<_> = Vec::with_capacity(pids.len());

        for pid in pids {
            let Ok(denylist) = query!(
                "
                select
                    since
                from denylist
                where initiator = $1 and other = $2
                ",
                pid,
                self.pid
            )
            .fetch_optional(&self.fm.db)
            .await
            else {
                warn!("user requested request setting of invalid user");
                continue;
            };

            if denylist.is_some() {
                blocked.push(PrincipalRequestBlockSetting {
                    data: Data {},
                    pid,
                    blocked: true,
                });
            } else {
                blocked.push(PrincipalRequestBlockSetting {
                    data: Data {},
                    pid,
                    blocked: false,
                });
            }
        }

        Ok(blocked)
    }
}

// unused?
// type HMacMd5 = hmac::Hmac<md5::Md5>;

impl Drop for FriendsUser {
    fn drop(&mut self) {
        let friends = mem::take(&mut self.maybe_remote_friend);
        let users = friends.into_inner();
        let pid = self.pid;
        let fm = self.fm.clone();
        tokio::spawn(async move {
            for user in users {
                let Some(user) = user.1.upgrade() else {
                    continue;
                };

                user.remote
                    .process_nintendo_notification_event_2(NintendoNotificationEvent {
                        event_type: 10,
                        sender: pid,
                        data: Any::new(&NintendoNotificationEventGeneral {
                            param3: DateTime::now().0,
                            ..Default::default()
                        })
                        .expect("type error"),
                    })
                    .await;
            }
            query!(
                "update nintendo_network_accounts set last_online = now() where pid = $1",
                pid
            )
            .execute(&fm.db)
            .await
            .ok();
        });
    }
}

impl AccountManagement for FriendsGuest {
    async fn nintendo_create_account(
        &self,
        principal_name: String,
        _key: String,
        _groups: u32,
        email: String,
        auth_data: Any,
    ) -> Result<(PID, String), ErrorCode> {
        let nex_token = if let Ok(extra_info) = auth_data.try_get_as::<AccountExtraInfo>() {
            extra_info.nex_token
        } else if let Ok(data) = auth_data.try_get_as::<NintendoCreateAccountData>() {
            data.nex_token
        } else {
            return Err(ErrorCode::Authentication_InvalidParam);
        };
        let (pid, nex_key) = nex_account::decode_nexact_token(&nex_token).map_err(|e| {
            error!("failed to decode token: {e}");
            info!("{nex_token:?}");
            ErrorCode::Authentication_InvalidParam
        })?;

        //let mac = derive_pid_hmac(data.nna_info.principal_basic_info.pid, &nexkey);

        let mut client = grpc_client().await.map_err(|e| {
            error!("error occurred in gRPC client: {e:?}");
            ErrorCode::Core_Unknown
        })?;

        let new_account: ActCreateInfo = ActCreateInfo {
            principal_name,
            key: nex_key.into(),
            email,
            pid,
        };

        let nex_key = client
            .create_new_sequential_or_update_and_get_account(new_account)
            .await
            .map_err(|_| ErrorCode::Core_Unknown)?
            .into_inner();

        if nex_key.key.len() != 16 {
            error!("nex key was not 16 bytes long");
            return Err(ErrorCode::Authentication_InvalidParam);
        }

        let nexkeyarray: [u8; 16] = nex_key.key.try_into().expect("how...?");

        let mac = derive_pid_hmac(pid, &nexkeyarray);

        let hex_str = hex::encode(mac);

        Ok((pid, hex_str))
    }
}
