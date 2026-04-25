use crate::nex::user::User;
use crate::rmc::protocols::notifications::{NotificationEvent, RemoteNotification};
use log::info;
use rand::random;
use rnex_core::PID;
use rnex_core::kerberos::KerberosDateTime;
use rnex_core::rmc::protocols::notifications::notification_types::{
    HOST_CHANGED, OWNERSHIP_CHANGED,
};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::response::ErrorCode::{Core_InvalidArgument, RendezVous_SessionVoid};
use rnex_core::rmc::structures::matchmake::gathering_flags::PERSISTENT_GATHERING;
use rnex_core::rmc::structures::matchmake::{
    Gathering, MatchmakeParam, MatchmakeSession, MatchmakeSessionSearchCriteria,
};
use rnex_core::rmc::structures::variant::Variant;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

pub struct MatchmakeManager {
    //pub gid_counter: AtomicU32,
    pub sessions: RwLock<HashMap<u32, Arc<Mutex<ExtendedMatchmakeSession>>>>,
    pub rv_cid_counter: AtomicU32,
    pub users: RwLock<HashMap<u32, Weak<User>>>,
}

impl MatchmakeManager {
    pub fn next_gid(&self) -> u32 {
        random()
        //self.gid_counter.fetch_add(1, Relaxed)
    }

    pub fn next_cid(&self) -> u32 {
        self.rv_cid_counter.fetch_add(1, Relaxed)
    }

    pub async fn get_session(
        &self,
        gid: u32,
    ) -> Result<Arc<Mutex<ExtendedMatchmakeSession>>, ErrorCode> {
        let sessions = self.sessions.read().await;

        let Some(session) = sessions.get(&gid) else {
            return Err(RendezVous_SessionVoid);
        };

        let session = session.clone();
        drop(sessions);

        Ok(session)
    }

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

    pub async fn initialize_garbage_collect_thread(this: Weak<Self>) {
        tokio::spawn(async move {
            while let Some(this) = this.upgrade() {
                this.garbage_collect().await;

                // every 30 minutes
                sleep(Duration::from_secs(60 * 30)).await;
            }
        });
    }
}

#[derive(Default, Debug)]
pub struct ExtendedMatchmakeSession {
    pub session: MatchmakeSession,
    pub connected_players: Vec<Weak<User>>,
}

fn read_bounds_string<T: FromStr>(str: &str) -> Option<(T, T)> {
    let bounds = str.split_once(",")?;

    Some((T::from_str(bounds.0).ok()?, T::from_str(bounds.1).ok()?))
}

fn check_bounds_str<T: FromStr + PartialOrd>(compare: T, str: &str) -> Option<bool> {
    let bounds: (T, T) = read_bounds_string(str)?;

    Some(bounds.0 <= compare && compare <= bounds.1)
}

pub async fn broadcast_notification<T: AsRef<User>>(
    players: &[T],
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
    pub fn get_active_players(&self) -> Vec<Arc<User>> {
        self.connected_players
            .iter()
            .filter_map(|u| u.upgrade())
            .collect()
    }

    #[inline(always)]
    pub async fn broadcast_notification(&self, notification_event: &NotificationEvent) {
        broadcast_notification(&self.get_active_players(), notification_event).await;
    }

    pub async fn from_matchmake_session(
        gid: u32,
        session: MatchmakeSession,
        host: &Weak<User>,
    ) -> Self {
        let Some(host) = host.upgrade() else {
            return Default::default();
        };

        let mm_session = MatchmakeSession {
            gathering: Gathering {
                self_gid: gid,
                owner_pid: host.pid,
                host_pid: host.pid,
                ..session.gathering.clone()
            },
            datetime: KerberosDateTime::now(),
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

        Self {
            session: mm_session,
            connected_players: Default::default(),
        }
    }

    pub async fn add_players(&mut self, conns: &[Weak<User>], join_msg: String) {
        let Some(initiating_user) = conns[0].upgrade() else {
            return;
        };

        let initiating_pid = initiating_user.pid;

        let old_particip = self.connected_players.clone();
        for conn in conns {
            self.connected_players.push(conn.clone());
        }
        self.session.participation_count = self.connected_players.len() as u32;

        for other_connection in &conns[1..] {
            let Some(other_conn) = other_connection.upgrade() else {
                continue;
            };

            let other_pid = other_conn.pid;
            /*if other_pid == self.session.gathering.owner_pid &&
                joining_pid == self.session.gathering.owner_pid{
                continue;
            }*/

            other_conn
                .remote
                .process_notification_event(NotificationEvent {
                    pid_source: initiating_pid,
                    notif_type: 122000,
                    param_1: self.session.gathering.self_gid as PID,
                    param_2: other_pid,
                    str_param: "".into(),
                    param_3: 0,
                })
                .await;
        }

        let list_of_connected_pids: Vec<_> = self
            .connected_players
            .iter()
            .filter_map(|p| p.upgrade())
            .map(|p| p.pid)
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
            for new_conn_pid in conns.iter().filter_map(Weak::upgrade).map(|c| c.pid) {
                old_conns
                    .remote
                    .process_notification_event(NotificationEvent {
                        pid_source: initiating_pid,
                        notif_type: 3001,
                        param_1: self.session.gathering.self_gid as PID,
                        param_2: new_conn_pid,
                        str_param: join_msg.clone(),
                        param_3: self.connected_players.len() as _,
                    })
                    .await;
            }
        }
    }

    pub fn has_active_players(&self) -> bool {
        self.connected_players
            .iter()
            .filter(|v| v.upgrade().is_some())
            .count()
            != 0
    }

    #[inline]
    pub fn is_reachable(&self) -> bool {
        (if self.session.gathering.flags & PERSISTENT_GATHERING != 0 {
            if self.has_active_players() {
                true
            } else {
                self.session.open_participation
            }
        } else {
            self.has_active_players()
        }) & self.has_active_players()
    }
    #[inline]
    pub fn is_joinable(&self) -> bool {
        self.is_reachable() && self.session.open_participation
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

        if !check_bounds_str(
            self.session.gathering.minimum_participants,
            &search_criteria.minimum_participants,
        )
        .ok_or(Core_InvalidArgument)?
        {
            return Ok(false);
        }

        if !check_bounds_str(
            self.session.gathering.maximum_participants,
            &search_criteria.maximum_participants,
        )
        .ok_or(Core_InvalidArgument)?
        {
            return Ok(false);
        }

        let game_mode: u32 = search_criteria
            .game_mode
            .parse()
            .map_err(|_| Core_InvalidArgument)?;

        if self.session.gamemode != game_mode {
            return Ok(false);
        }

        let mm_sys_type: u32 = search_criteria
            .matchmake_system_type
            .parse()
            .map_err(|_| Core_InvalidArgument)?;

        if self.session.matchmake_system_type != mm_sys_type {
            return Ok(false);
        }

        if search_criteria
            .attribs
            .get(0)
            .map(|str| str.parse().ok())
            .flatten()
            != self.session.attributes.get(0).map(|v| *v)
        {
            return Ok(false);
        }
        if search_criteria
            .attribs
            .get(2)
            .map(|str| str.parse().ok())
            .flatten()
            != self.session.attributes.get(2).map(|v| *v)
        {
            return Ok(false);
        }
        if search_criteria
            .attribs
            .get(3)
            .map(|str| str.parse().ok())
            .flatten()
            != self.session.attributes.get(3).map(|v| *v)
        {
            return Ok(false);
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
            .find(|p| p.pid != self.session.gathering.owner_pid)
        else {
            self.session.gathering.owner_pid = 0;

            return Ok(());
        };

        self.session.gathering.owner_pid = new_owner.pid;

        self.broadcast_notification(&NotificationEvent {
            pid_source: initiator_pid,
            notif_type: OWNERSHIP_CHANGED,
            param_1: self.session.gathering.self_gid as PID,
            param_2: new_owner.pid,
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
            .retain(|u| u.upgrade().is_some_and(|u| u.pid != pid));

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
