use rnex_mm_protos::{RemoteMatchMakingClientProtocol, notifications::RemoteNotification};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        Arc, Weak,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use cfg_if::cfg_if;
use rand::random;
use rnex_mm_protos::{
    matchmake::{
        Gathering, MatchmakeSession, MatchmakeSessionSearchCriteria,
        gathering_flags::PERSISTENT_GATHERING,
    },
    notifications::{
        NotificationEvent,
        notification_types::{HOST_CHANGED, OWNERSHIP_CHANGED},
    },
};
use rnex_rmc::{RmcPureRemoteObject, response::ErrorCode};
use rnex_server::{
    ConnectionInitData, PassthroughInitModule, RnexManager, WeakPassthroughInitModule,
};
use rnex_util::PID;
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};
use tracing::{info, instrument};

use crate::user::MatchmakeUser;

#[derive(Debug)]
pub struct MatchmakeManager {
    //pub gid_counter: AtomicU32,
    pub sessions: RwLock<HashMap<u32, Arc<Mutex<ExtendedMatchmakeSession>>>>,
    pub rv_cid_counter: AtomicU32,
    pub users: RwLock<HashMap<u32, WeakPassthroughInitModule<MatchmakeUser>>>,
    pub users_by_pid: RwLock<HashMap<PID, WeakPassthroughInitModule<MatchmakeUser>>>,
}

impl MatchmakeManager {
    pub fn next_gid(&self) -> u32 {
        random()
        //self.gid_counter.fetch_add(1, Relaxed)
    }

    pub fn next_cid(&self) -> u32 {
        self.rv_cid_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[instrument]
    pub async fn get_session(
        &self,
        gid: u32,
    ) -> Result<Arc<Mutex<ExtendedMatchmakeSession>>, ErrorCode> {
        let sessions = self.sessions.read().await;

        let Some(session) = sessions.get(&gid) else {
            return Err(ErrorCode::RendezVous_SessionVoid);
        };

        let session = session.clone();
        drop(sessions);

        Ok(session)
    }

    #[instrument]
    async fn garbage_collect(&self) {
        info!("running rnex garbage collector over all sessions and users");

        let mut idx = 0;

        let mut to_be_deleted_gids = Vec::new();

        // i am very well aware of how inefficient doing it like this is but this is the only
        // way which i could think of to do this without potentially causing a deadlock of
        // the entire server
        while let Some((gid, session)) = {
            let sessions = self.sessions.read().await;
            let session_pair = sessions.iter().nth(idx).map(|s| (*s.0, s.1.clone()));
            drop(sessions);

            session_pair
        } {
            let session = session.lock().await;

            if !session.is_reachable() {
                to_be_deleted_gids.push(gid);
            }

            idx += 1;
        }

        let mut sessions = self.sessions.write().await;

        for gid in to_be_deleted_gids {
            sessions.remove(&gid);
        }
    }

    #[instrument]
    pub fn initialize_garbage_collect_thread(this: Weak<Self>) {
        tokio::spawn(async move {
            while let Some(this) = this.upgrade() {
                this.garbage_collect().await;

                // every 5 minutes
                sleep(Duration::from_secs(60 * 5)).await;
            }
        });
    }

    // this could be far more efficient but it is INCREDIBLY difficult to iterate over something
    // asyncronously propperly
    #[instrument]
    pub async fn search_by_criteria(
        &self,
        criterias: &[MatchmakeSessionSearchCriteria],
    ) -> Result<Vec<Arc<Mutex<ExtendedMatchmakeSession>>>, ErrorCode> {
        let sessions = self.sessions.read().await;
        let mut list = Vec::with_capacity(sessions.len());
        for session in sessions.values() {
            let inner_session = session.lock().await;
            if !inner_session.is_joinable() {
                continue;
            }

            let mut bool_matched_criteria = false;

            for criteria in criterias {
                if inner_session.matches_criteria(criteria)? {
                    bool_matched_criteria = true;
                }
            }

            if bool_matched_criteria {
                info!("matched session: {:?}", session);
                list.push(session.clone());
            }
        }

        drop(sessions);
        Ok(list)
    }
}

#[derive(Default, Debug)]
pub struct ExtendedMatchmakeSession {
    pub session: MatchmakeSession,
    pub connected_players: Vec<WeakPassthroughInitModule<MatchmakeUser>>,
}

fn read_bounds_string<T: FromStr>(str: &str) -> Option<(T, T)> {
    let bounds = str.split_once(",")?;

    Some((T::from_str(bounds.0).ok()?, T::from_str(bounds.1).ok()?))
}

fn check_bounds_str<T: FromStr + PartialOrd>(compare: T, str: &str) -> Option<bool> {
    if let Some(bounds) = read_bounds_string::<T>(str) {
        return Some(bounds.0 <= compare && compare <= bounds.1);
    }
    if let Ok(val) = T::from_str(str) {
        return Some(val == compare);
    }
    if str.is_empty() {
        return Some(true);
    }
    None
}

pub async fn broadcast_notification<T: AsRef<MatchmakeUser>>(
    players: impl Iterator<Item = T>,
    notification_event: &NotificationEvent,
) {
    for player in players {
        let player = player.as_ref();
        player
            .remote
            .process_notification_event(notification_event.clone())
            .await;
    }
}

impl ExtendedMatchmakeSession {
    #[inline(always)]
    pub fn get_active_players(&self) -> impl Iterator<Item = PassthroughInitModule<MatchmakeUser>> {
        self.connected_players.iter().filter_map(|u| u.upgrade())
    }

    #[inline(always)]
    pub async fn broadcast_notification(&self, notification_event: &NotificationEvent) {
        broadcast_notification(self.get_active_players(), notification_event).await;
    }

    pub async fn from_matchmake_session(
        gid: u32,
        session: MatchmakeSession,
        host: &WeakPassthroughInitModule<MatchmakeUser>,
    ) -> Self {
        let Some(host) = host.upgrade() else {
            return Default::default();
        };

        cfg_if! {
            if #[cfg(feature = "v3-5-0")]{
                use rnex_rmc::variant::Variant;
                use rnex_util::date_time::DateTime;
                use rnex_mm_protos::matchmake::MatchmakeParam;
                let mm_session = MatchmakeSession {
                    gathering: Gathering {
                        self_gid: gid,
                        owner_pid: host.base.pid,
                        host_pid: host.base.pid,
                        ..session.gathering.clone()
                    },
                    datetime: DateTime::now(),
                    session_key: (0..32).map(|_| random()).collect(),
                    matchmake_param: MatchmakeParam {
                        params: vec![
                            ("@SR".to_owned(), Variant::Bool(true)),
                            ("@GIR".to_owned(), Variant::SInt64(3)),
                        ],
                    },
                    system_password_enabled: false,
                    ..session
                };

                return Self {
                    session: mm_session,
                    connected_players: Default::default(),
                }
            } else {
                let mm_session = MatchmakeSession {
                    gathering: Gathering {
                        self_gid: gid,
                        owner_pid: host.base.pid,
                        host_pid: host.base.pid,
                        ..session.gathering.clone()
                    },
                    session_key: (0..32).map(|_| random()).collect(),
                    ..session
                };
                return Self {
                    session: mm_session,
                    connected_players: Default::default(),
                }
            }
        }
    }

    pub async fn add_players(
        &mut self,
        conns: &[WeakPassthroughInitModule<MatchmakeUser>],
        join_msg: String,
    ) {
        let Some(initiating_user) = conns[0].upgrade() else {
            return;
        };

        let initiating_pid = initiating_user.base.pid;

        let old_particip = self.connected_players.clone();
        for conn in conns {
            self.connected_players.push(conn.clone());
        }
        self.session.participation_count = self.connected_players.len() as u32;

        for other_connection in &conns[1..] {
            let Some(other_conn) = other_connection.upgrade() else {
                continue;
            };

            let other_pid = other_conn.base.pid;
            /*if other_pid == self.session.gathering.owner_pid &&
                joining_pid == self.session.gathering.owner_pid{
                continue;
            }*/

            other_conn
                .remote
                .process_notification_event(NotificationEvent {
                    pid_source: initiating_pid,
                    notif_type: 122_000,
                    param_1: self.session.gathering.self_gid as PID,
                    param_2: other_pid,
                    str_param: "".into(),
                    #[cfg(feature = "third-notif-param")]
                    param_3: 0,
                })
                .await;
        }

        let list_of_connected_pids: Vec<_> = self
            .connected_players
            .iter()
            .filter_map(|p| p.upgrade())
            .map(|p| p.base.pid)
            .collect();

        for other_connection in conns {
            let Some(other_conn) = other_connection.upgrade() else {
                continue;
            };

            // let other_pid = other_conn.pid;
            /*if other_pid == self.session.gathering.owner_pid &&
                joining_pid == self.session.gathering.owner_pid{
                continue;
            }*/

            for pid in &list_of_connected_pids {
                other_conn
                    .remote
                    .process_notification_event(NotificationEvent {
                        pid_source: initiating_pid,
                        notif_type: 3001,
                        param_1: self.session.gathering.self_gid as PID,
                        param_2: *pid,
                        str_param: join_msg.clone(),
                        #[cfg(feature = "third-notif-param")]
                        param_3: self.connected_players.len() as _,
                    })
                    .await;
            }
        }

        for old_conns in &old_particip {
            let Some(old_conns) = old_conns.upgrade() else {
                continue;
            };
            /*if old_conns.pid != self.session.gathering.host_pid {
                continue;
            }*/
            for new_conn_pid in conns
                .iter()
                .filter_map(WeakPassthroughInitModule::upgrade)
                .map(|c| c.base.pid)
            {
                old_conns
                    .remote
                    .process_notification_event(NotificationEvent {
                        pid_source: initiating_pid,
                        notif_type: 3001,
                        param_1: self.session.gathering.self_gid as PID,
                        param_2: new_conn_pid,
                        str_param: join_msg.clone(),
                        #[cfg(feature = "third-notif-param")]
                        param_3: self.connected_players.len() as _,
                    })
                    .await;
            }
        }
    }

    pub fn has_min_active_players(&self) -> bool {
        self.connected_players
            .iter()
            .filter(|v| v.upgrade().is_some())
            .count()
            >= self.session.gathering.minimum_participants as _
    }

    #[inline]
    pub fn get_host(&self) -> Option<PassthroughInitModule<MatchmakeUser>> {
        self.get_active_players()
            .find(|v| v.base.pid == self.session.gathering.host_pid)
    }

    #[inline]
    pub fn is_reachable(&self) -> bool {
        self.get_active_players()
            .any(|v| v.base.pid == self.session.gathering.host_pid)
            && (if self.session.gathering.flags & PERSISTENT_GATHERING != 0 {
                if self.has_min_active_players() {
                    true
                } else {
                    self.session.open_participation
                }
            } else {
                self.has_min_active_players()
            }) & self.has_min_active_players()
    }
    #[inline]
    pub fn is_joinable(&self) -> bool {
        #[cfg(not(feature = "splatoon"))]
        let is_open = self.session.open_participation;
        #[cfg(feature = "splatoon")]
        let is_open = if self.session.gamemode == 12 {
            true
        } else {
            self.session.open_participation
        };
        self.is_reachable() && is_open
    }

    pub fn matches_criteria(
        &self,
        search_criteria: &MatchmakeSessionSearchCriteria,
    ) -> Result<bool, ErrorCode> {
        // todo: implement the rest of the search criteria

        if search_criteria.vacant_only {
            if (self.connected_players.len() as u16 + search_criteria.vacant_participants)
                > self.session.gathering.maximum_participants
            {
                return Ok(false);
            }
        }

        if search_criteria.exclude_locked {
            if !self.session.open_participation {
                return Ok(false);
            }
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "v3-5-0")]{
                if search_criteria.exclude_system_password_set {
                    if self.session.system_password_enabled {
                        return Ok(false);
                    }
                }

                if search_criteria.exclude_user_password_set {
                    if self.session.user_password_enabled {
                        return Ok(false);
                    }
                }
            }
        }

        if !check_bounds_str(
            self.session.gathering.minimum_participants,
            &search_criteria.minimum_participants,
        )
        .ok_or(ErrorCode::Core_InvalidArgument)?
        {
            return Ok(false);
        }

        if !check_bounds_str(
            self.session.gathering.maximum_participants,
            &search_criteria.maximum_participants,
        )
        .ok_or(ErrorCode::Core_InvalidArgument)?
        {
            return Ok(false);
        }

        let game_mode: u32 = search_criteria
            .game_mode
            .parse()
            .map_err(|_| ErrorCode::Core_InvalidArgument)?;

        if self.session.gamemode != game_mode {
            return Ok(false);
        }

        let mm_sys_type: u32 = search_criteria
            .matchmake_system_type
            .parse()
            .map_err(|_| ErrorCode::Core_InvalidArgument)?;

        if self.session.matchmake_system_type != mm_sys_type {
            return Ok(false);
        }

        #[cfg(feature = "splatoon")]
        {
            if !search_criteria.attribs.get(0).is_some_and(|s| {
                self.session
                    .attributes
                    .get(0)
                    .is_some_and(|a| s.0.contains(a))
            }) {
                return Ok(false);
            }
            if !search_criteria.attribs.get(2).is_some_and(|s| {
                self.session
                    .attributes
                    .get(2)
                    .is_some_and(|a| s.0.contains(a))
            }) {
                return Ok(false);
            }
            if !search_criteria.attribs.get(3).is_some_and(|s| {
                self.session
                    .attributes
                    .get(3)
                    .is_some_and(|a| s.0.contains(a))
            }) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn migrate_ownership(&mut self, initiator_pid: PID) -> Result<(), ErrorCode> {
        let players: Vec<_> = self
            .connected_players
            .iter()
            .filter_map(|p| p.upgrade())
            .collect();

        let Some(new_owner) = players
            .iter()
            .find(|p| p.base.pid != self.session.gathering.owner_pid)
        else {
            self.session.gathering.owner_pid = 0;

            return Ok(());
        };

        self.session.gathering.owner_pid = new_owner.base.pid;

        self.broadcast_notification(&NotificationEvent {
            pid_source: initiator_pid,
            notif_type: OWNERSHIP_CHANGED,
            param_1: self.session.gathering.self_gid as PID,
            param_2: new_owner.base.pid,
            ..Default::default()
        })
        .await;

        Ok(())
    }

    pub async fn migrate_host(&mut self, initiator_pid: PID) -> Result<(), ErrorCode> {
        // let players: Vec<_> = self.connected_players.iter().filter_map(|p| p.upgrade()).collect();

        self.session.gathering.host_pid = self.session.gathering.owner_pid;

        self.broadcast_notification(&NotificationEvent {
            pid_source: initiator_pid,
            notif_type: HOST_CHANGED,
            param_1: self.session.gathering.self_gid as PID,
            ..Default::default()
        })
        .await;

        Ok(())
    }

    pub async fn remove_player_from_session(
        &mut self,
        pid: PID,
        message: &str,
    ) -> Result<(), ErrorCode> {
        self.connected_players
            .retain(|u| u.upgrade().is_some_and(|u| u.base.pid != pid));

        self.session.participation_count =
            (self.connected_players.len() & u32::MAX as usize) as u32;

        if pid == self.session.gathering.owner_pid {
            self.migrate_ownership(pid).await?;
        }

        if pid == self.session.gathering.host_pid {
            self.migrate_host(pid).await?;
        }

        // todo: support DisconnectChangeOwner

        // todo: finish the rest of this

        for player in self.connected_players.iter().filter_map(|p| p.upgrade()) {
            player
                .remote
                .process_notification_event(NotificationEvent {
                    notif_type: 3008,
                    pid_source: pid,
                    param_1: self.session.gathering.self_gid as PID,
                    param_2: pid,
                    str_param: message.to_owned(),
                    ..Default::default()
                })
                .await;
        }

        Ok(())
    }
}

impl RnexManager for MatchmakeManager {
    type User = MatchmakeUser;

    type InitData = ConnectionInitData;

    async fn init_new_user(
        this: PassthroughInitModule<Self>,
        mod_holder: &rnex_server::ModuleHolder,
        remote: &rnex_rmc::RmcConnection,
        _: &Self::InitData,
        user: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        MatchmakeUser {
            base: mod_holder
                .get_ref_init_pt()
                .expect("matchmaking module cannot work without the base module"),
            matchmake_manager: this,
            remote: RemoteMatchMakingClientProtocol::new(remote.clone()),
            this: user,
        }
    }
    async fn post_init(user: &Self::User) {
        let mut users = user.matchmake_manager.users.write().await;
        users.insert(user.base.cid, user.this.clone());
        drop(users);
        let mut users = user.matchmake_manager.users_by_pid.write().await;
        users.insert(user.base.pid, user.this.clone());
        drop(users);
    }
}
