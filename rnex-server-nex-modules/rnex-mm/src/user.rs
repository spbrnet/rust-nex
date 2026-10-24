use std::sync::Arc;

use crate::matchmake::{ExtendedMatchmakeSession, MatchmakeManager};
use rnex_base::user::BaseUser;
use rnex_base_protos::ResultsRange;
use rnex_mm_protos::matchmake::gathering_flags;
use rnex_mm_protos::{
    LocalMatchMakingProtocol, RemoteMatchMakingClientProtocol,
    matchmake::{
        AutoMatchmakeParam, CreateMatchmakeSessionParam, Gathering, JoinMatchmakeSessionParam,
        Matchmake, MatchmakeSession, MatchmakeSessionSearchCriteria, ParticipantDetails,
        PlayingSession, SimplePlayingSession,
    },
    matchmake_ext::MatchmakeExt,
    matchmake_extension::MatchmakeExtension,
    nat_traversal::NatTraversal,
    notifications::{
        NotificationEvent, RemoteNotification,
        notification_types::{END_GATHERING, REQUEST_JOIN_GATHERING},
    },
};
use rnex_rmc::{
    HasRmcConnection, any::Any, message::RMCMessage, response::ErrorCode, rmc_struct,
    serialization::RmcSerialize,
};
use rnex_server::{PassthroughInitModule, WeakPassthroughInitModule};
#[cfg(feature = "nx")]
use rnex_util::station_url::nat_types::PUBLIC;
use rnex_util::{
    PID,
    diagnostics,
    station_url::{StationUrl, UrlOptions},
};
#[cfg(feature = "splatoon1")]
use std::collections::HashSet;
use tokio::sync::Mutex;
use tracing::{info, warn};

/*cfg_if! {
    if #[cfg(feature = "datastore")] {
        use rnex_core::rmc::protocols::datastore::{DataStore, RawDataStore, RawDataStoreInfo, RemoteDataStore};
        define_rmc_proto!(
            proto UserProtocol{
                Secure,
                MatchmakeExtension,
                MatchmakeExt,
                Matchmake,
                NatTraversal,
                Ranking,
                Utility,
                DataStore,
                MessageDelivery
            }
        );
    } else {
        define_rmc_proto!(
            proto UserProtocol{
                Secure,
                MatchmakeExtension,
                MatchmakeExt,
                Matchmake,
                NatTraversal,
                Utility,
                Ranking,
                MessageDelivery
            }
        );
    }
}*/

/// Connection tickets are allowances to join a specific lobby, they are given out as soon as nat checks pass,
/// there are 2 stages of tickets because both sides have to do nat checking before we let the player join
/// the lobby
pub struct ConnectionTicket {
    pub cid: u32,
    pub result: bool,
}

#[derive(Debug)]
#[rmc_struct(MatchMakingProtocol)]
pub struct MatchmakeUser {
    pub base: PassthroughInitModule<BaseUser>,
    /*
    pub pid: PID,
    pub cid: u32,
    pub ip: PRUDPSockAddr,
    */
    pub this: WeakPassthroughInitModule<MatchmakeUser>,
    pub remote: RemoteMatchMakingClientProtocol,
    //pub station_url: RwLock<Vec<StationUrl>>,
    pub matchmake_manager: PassthroughInitModule<MatchmakeManager>,
}

impl MatchmakeExtension for MatchmakeUser {
    async fn close_participation(&self, gid: u32) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;
        if session.session.gathering.host_pid != self.base.pid
            && session.session.gathering.owner_pid != self.base.pid
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        session.session.open_participation = false;

        Ok(())
    }

    async fn open_participation(&self, gid: u32) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;
        if session.session.gathering.host_pid != self.base.pid
            && session.session.gathering.owner_pid != self.base.pid
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        session.session.open_participation = true;

        Ok(())
    }

    async fn browse_matchmake_session(
        &self,
        browse_criteria: MatchmakeSessionSearchCriteria,
        result_range: ResultsRange,
    ) -> Result<Vec<Any<Gathering>>, ErrorCode> {
        println!(
            "browsing matchmake sessions with criteria: {:?}",
            browse_criteria
        );

        let results = self
            .matchmake_manager
            .search_by_criteria(&[browse_criteria])
            .await?;

        let mm_list = result_range.make_from_list(&results[..]);
        println!("filtered mm_list count: {}", mm_list.len());

        let mut list = Vec::with_capacity(mm_list.len());

        for mm_sess in &mm_list {
            let mm_sess_guard = mm_sess.lock().await;

            let gathering_any =
                Any::new(&mm_sess_guard.session).map_err(|_| ErrorCode::Core_InvalidArgument)?;
            list.push(gathering_any);
        }

        println!(
            "successfully collected {} sessions into output list",
            list.len()
        );

        Ok(list)
    }

    async fn get_playing_session(&self, pids: Vec<PID>) -> Result<Vec<PlayingSession>, ErrorCode> {
        let mut found = Vec::new();
        for pid in pids {
            if let Some(session) = self.matchmake_manager.playing_session_for(pid).await {
                found.push(PlayingSession {
                    pid,
                    gathering: Any::new(&session).map_err(|_| ErrorCode::Core_InvalidArgument)?,
                });
            }
        }
        info!(
            caller_pid = self.base.pid,
            count = found.len(),
            "playing sessions found"
        );
        Ok(found)
    }

    async fn get_simple_playing_session(
        &self,
        mut pids: Vec<PID>,
        include_login_user: bool,
    ) -> Result<Vec<SimplePlayingSession>, ErrorCode> {
        if include_login_user && !pids.contains(&self.base.pid) {
            pids.push(self.base.pid);
        }
        let mut found = Vec::new();
        for pid in pids {
            if pid == self.base.pid && !include_login_user {
                continue;
            }
            if let Some(session) = self.matchmake_manager.playing_session_for(pid).await {
                found.push(SimplePlayingSession {
                    pid,
                    gid: session.gathering.self_gid,
                    game_mode: session.gamemode,
                    attribute: session.attributes.first().copied().unwrap_or_default(),
                });
            }
        }
        info!(
            caller_pid = self.base.pid,
            count = found.len(),
            "simple playing sessions found"
        );
        Ok(found)
    }

    #[cfg(feature = "v3-5-0")]
    async fn update_progress_score(&self, gid: u32, progress: u8) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;
        if session.session.gathering.host_pid != self.base.pid
            && session.session.gathering.owner_pid != self.base.pid
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        session.session.progress_score = progress;

        Ok(())
    }

    async fn create_matchmake_session_with_param(
        &self,
        create_session_param: CreateMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        let gid = self.matchmake_manager.next_gid();

        let mut new_session = ExtendedMatchmakeSession::from_matchmake_session(
            gid,
            create_session_param.matchmake_session,
            &self.this.clone(),
        )
        .await;

        let mut joining_players = vec![self.this.clone()];
        #[cfg(feature = "splatoon1")]
        let mut expected_result_roster = None;

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(create_session_param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;
            if !old_gathering
                .get_active_players()
                .any(|p| p.base.pid == self.base.pid)
            {
                return Err(ErrorCode::RendezVous_PermissionDenied);
            }
            #[cfg(feature = "splatoon1")]
            {
                let parent_roster: HashSet<PID> = old_gathering
                    .get_active_players()
                    .map(|player| player.base.pid)
                    .collect();
                if create_session_param.gid_for_participation_check != 0
                    && parent_roster.len() == 8
                    && create_session_param
                        .additional_participants
                        .iter()
                        .all(|pid| parent_roster.contains(pid))
                {
                    expected_result_roster = Some(parent_roster);
                }
            }
            let players = old_gathering
                .connected_players
                .iter()
                .filter_map(|v| v.upgrade())
                .filter(|u| {
                    create_session_param
                        .additional_participants
                        .iter()
                        .any(|p| *p == u.base.pid)
                });
            for player in players {
                joining_players.push(PassthroughInitModule::downgrade(&player));
            }
        }

        drop(users);

        new_session.session.participation_count = create_session_param.participation_count as u32;
        new_session
            .add_players(&joining_players, create_session_param.join_message)
            .await?;

        #[cfg(feature = "splatoon1")]
        let actual_child_roster: HashSet<PID> = new_session
            .get_active_players()
            .map(|player| player.base.pid)
            .collect();

        let session = new_session.session.clone();

        let mut sessions = self.matchmake_manager.sessions.write().await;
        if sessions.len() >= 1024 {
            return Err(ErrorCode::Core_InvalidArgument);
        }
        sessions.insert(gid, Arc::new(Mutex::new(new_session)));
        drop(sessions);

        #[cfg(feature = "splatoon1")]
        if let Some(expected_result_roster) = expected_result_roster {
            if let Err(error) = self
                .matchmake_manager
                .results
                .observe_child_created(
                    gid,
                    session.gamemode,
                    expected_result_roster,
                    actual_child_roster,
                )
                .await
            {
                tracing::error!(?error, "could not record splatoon1 child gathering");
            }
        }

        Ok(session)
    }

    async fn join_matchmake_session_with_param(
        &self,
        join_session_param: JoinMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        let session = self
            .matchmake_manager
            .get_session(join_session_param.gid)
            .await?;

        let mut joining_players = vec![self.this.clone()];

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(join_session_param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;
            if !old_gathering
                .get_active_players()
                .any(|p| p.base.pid == self.base.pid)
            {
                return Err(ErrorCode::RendezVous_PermissionDenied);
            }
            let players = old_gathering
                .connected_players
                .iter()
                .filter_map(|v| v.upgrade())
                .filter(|u| {
                    join_session_param
                        .additional_participants
                        .iter()
                        .any(|p| *p == u.base.pid)
                });
            for player in players {
                joining_players.push(PassthroughInitModule::downgrade(&player));
            }
        }

        drop(users);
        let mut session = session.lock().await;
        if !session.session.open_participation
            && !session
                .get_active_players()
                .any(|p| p.base.pid == self.base.pid)
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        #[cfg(feature = "v3-5-0")]
        if join_session_param.user_password != session.session.user_password {
            return Err(ErrorCode::RendezVous_MatchmakeSessionUserPasswordUnmatch);
        }
        session
            .add_players(&joining_players, join_session_param.join_message)
            .await?;

        let mm_session = session.session.clone();
        #[cfg(feature = "splatoon1")]
        let actual_child_roster: HashSet<PID> = session
            .get_active_players()
            .map(|player| player.base.pid)
            .collect();
        drop(session);

        #[cfg(feature = "splatoon1")]
        if let Err(error) = self
            .matchmake_manager
            .results
            .observe_child_join(join_session_param.gid, actual_child_roster)
            .await
        {
            tracing::error!(?error, "could not confirm splatoon1 child gathering");
        }

        Ok(mm_session)
    }

    async fn auto_matchmake_with_param_postpone(
        &self,
        param: AutoMatchmakeParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        let mut joining_players = vec![self.this.clone()];
        #[cfg(feature = "splatoon1")]
        let mut joining_parent_verified = false;

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;
            if !old_gathering
                .get_active_players()
                .any(|p| p.base.pid == self.base.pid)
            {
                return Err(ErrorCode::RendezVous_PermissionDenied);
            }
            #[cfg(feature = "splatoon1")]
            {
                let parent_pids: HashSet<PID> = old_gathering
                    .get_active_players()
                    .map(|player| player.base.pid)
                    .collect();
                joining_parent_verified = param.gid_for_participation_check != 0
                    && param
                        .additional_participants
                        .iter()
                        .all(|pid| parent_pids.contains(pid));
            }
            let players = old_gathering
                .connected_players
                .iter()
                .filter_map(|v| v.upgrade())
                .filter(|u| {
                    param
                        .additional_participants
                        .iter()
                        .any(|p| *p == u.base.pid)
                });
            for player in players {
                joining_players.push(PassthroughInitModule::downgrade(&player));
            }
        }

        drop(users);
        #[cfg(feature = "splatoon1")]
        let joining_roster: HashSet<PID> = joining_players
            .iter()
            .filter_map(WeakPassthroughInitModule::upgrade)
            .map(|player| player.base.pid)
            .collect();

        let sessions = self
            .matchmake_manager
            .search_by_criteria(&param.search_criteria[..])
            .await?;

        if let Some(session) = sessions.get(0) {
            let mut session = session.lock().await;
            #[cfg(feature = "splatoon1")]
            let existing_roster: HashSet<PID> = session
                .get_active_players()
                .map(|player| player.base.pid)
                .collect();
            session
                .add_players(&joining_players, param.join_message)
                .await?;
            #[cfg(feature = "splatoon1")]
            let final_roster: HashSet<PID> = session
                .get_active_players()
                .map(|player| player.base.pid)
                .collect();
            let matched = session.session.clone();
            drop(session);
            #[cfg(feature = "splatoon1")]
            if joining_parent_verified {
                if let Err(error) = self
                    .matchmake_manager
                    .results
                    .record_automatic_match(
                        matched.gathering.self_gid,
                        matched.gamemode,
                        existing_roster,
                        joining_roster,
                        final_roster,
                    )
                    .await
                {
                    tracing::error!(?error, "could not record splatfest match");
                }
            }
            return Ok(matched);
        }

        drop(sessions);

        println!("making new session!");

        let AutoMatchmakeParam {
            join_message,
            participation_count,
            gid_for_participation_check,
            matchmake_session,
            additional_participants,
            ..
        } = param;

        self.create_matchmake_session_with_param(CreateMatchmakeSessionParam {
            join_message,
            participation_count,
            gid_for_participation_check,
            create_matchmake_session_option: 0,
            matchmake_session,
            additional_participants,
        })
        .await
    }

    async fn find_matchmake_session_by_gathering_id_detail(
        &self,
        gid: u32,
    ) -> Result<MatchmakeSession, ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let session = session.lock().await;

        Ok(session.session.clone())
    }

    async fn modify_current_game_attribute(
        &self,
        gid: u32,
        attrib_index: u32,
        attrib_val: u32,
    ) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let mut session = session.lock().await;

        if session.session.gathering.host_pid != self.base.pid
            && session.session.gathering.owner_pid != self.base.pid
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        *session
            .session
            .attributes
            .get_mut(attrib_index as usize)
            .ok_or(ErrorCode::Core_InvalidArgument)? = attrib_val;

        Ok(())
    }

    async fn create_matchmake_session(
        &self,
        gathering: Any<Gathering>,
        message: String,
    ) -> Result<(u32, Vec<u8>), ErrorCode> {
        let session: MatchmakeSession = gathering.try_get_as()?;

        let session = self
            .create_matchmake_session_with_param(CreateMatchmakeSessionParam {
                matchmake_session: session,
                additional_participants: vec![],
                gid_for_participation_check: 0,
                create_matchmake_session_option: 0,
                join_message: message,
                participation_count: 1,
            })
            .await?;

        Ok((session.gathering.self_gid, session.session_key))
    }

    async fn get_friend_notification_data(
        &self,
        _ty: i32,
    ) -> Result<Vec<NotificationEvent>, ErrorCode> {
        Ok(vec![])
    }
    async fn update_notification_data(
        &self,
        ty: u32,
        param_1: u32,
        param_2: u32,
        str_param: String,
    ) -> Result<(), ErrorCode> {
        let recpipent = param_2;
        let Some(user) = self
            .matchmake_manager
            .users_by_pid
            .read()
            .await
            .get(&bytemuck::cast(recpipent))
            .and_then(|v| v.upgrade())
        else {
            return Err(ErrorCode::Core_InvalidArgument);
        };
        info!("notif ty : {}", ty);
        match ty {
            REQUEST_JOIN_GATHERING => {
                user.remote
                    .process_notification_event(NotificationEvent {
                        pid_source: self.base.pid,
                        notif_type: REQUEST_JOIN_GATHERING * 1000,
                        param_1: bytemuck::cast(param_1),
                        param_2: bytemuck::cast(param_2),
                        #[cfg(feature = "third-notif-param")]
                        param_3: 0,
                        str_param,
                    })
                    .await;
            }
            END_GATHERING => {
                user.remote
                    .process_notification_event(NotificationEvent {
                        pid_source: self.base.pid,
                        notif_type: END_GATHERING * 1000,
                        param_1: bytemuck::cast(param_1),
                        param_2: bytemuck::cast(param_2),
                        #[cfg(feature = "third-notif-param")]
                        param_3: 0,
                        str_param,
                    })
                    .await;
            }
            _ => {
                return Err(ErrorCode::Core_InvalidArgument);
            }
        }
        Ok(())
    }

    async fn update_application_buffer(
        &self,
        gid: u32,
        application_buffer: Vec<u8>,
    ) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;

        if session.session.gathering.host_pid != self.base.pid
            && session.session.gathering.owner_pid != self.base.pid
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }

        session.session.application_buffer = application_buffer;

        Ok(())
    }

    async fn join_matchmake_session_ex(
        &self,
        gid: u32,
        message: String,
        _dont_care_block_list: bool,
        //participation_count: u16,
    ) -> Result<Vec<u8>, ErrorCode> {
        let sess = self.matchmake_manager.get_session(gid).await?;
        let mut sess = sess.lock().await;
        if !sess.session.open_participation
            && !sess
                .get_active_players()
                .any(|p| p.base.pid == self.base.pid)
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        sess.add_players(&[self.this.clone()], message).await?;
        let session_key = sess.session.session_key.clone();
        #[cfg(feature = "splatoon1")]
        let actual_child_roster: HashSet<PID> = sess
            .get_active_players()
            .map(|player| player.base.pid)
            .collect();
        drop(sess);

        #[cfg(feature = "splatoon1")]
        if let Err(error) = self
            .matchmake_manager
            .results
            .observe_child_join(gid, actual_child_roster)
            .await
        {
            tracing::error!(?error, "could not confirm splatoon1 child gathering");
        }

        Ok(session_key)
    }

    async fn auto_matchmake_with_search_criteria_postpone(
        &self,
        criteria: Vec<MatchmakeSessionSearchCriteria>,
        gathering: Any<Gathering>,
        join_message: String,
    ) -> Result<Any<Gathering>, ErrorCode> {
        let session: MatchmakeSession = gathering.try_get_as()?;

        println!("automm criteria: {:?}", criteria);

        let session = self
            .auto_matchmake_with_param_postpone(AutoMatchmakeParam {
                matchmake_session: session,
                additional_participants: vec![],
                gid_for_participation_check: 0,
                auto_matchmake_option: 0,
                join_message,
                participation_count: 0,
                search_criteria: criteria,
                target_gids: vec![],
            })
            .await?;

        let any = Any::new(&session).map_err(|_| ErrorCode::Core_SystemError)?;

        Ok(any)
    }
}

impl Matchmake for MatchmakeUser {
    async fn find_by_single_id(&self, gid: u32) -> Result<(bool, Any<Gathering>), ErrorCode> {
        let s = self.matchmake_manager.get_session(gid).await?;
        let s = s.lock().await;
        Ok((
            true,
            Any::new(&s.session).map_err(|_| ErrorCode::Custom_Unknown)?,
        ))
    }

    async fn unregister_gathering(&self, _gid: u32) -> Result<bool, ErrorCode> {
        Ok(true)
    }
    async fn get_session_urls(&self, gid: u32) -> Result<Vec<StationUrl>, ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let session = session.lock().await;

        let urls: Vec<_> = session
            .connected_players
            .iter()
            .filter_map(|v| v.upgrade())
            .filter(|u| u.base.pid == session.session.gathering.host_pid)
            .map(|u| async move { u.base.station_url.read().await.clone() })
            .next()
            .ok_or(ErrorCode::RendezVous_SessionClosed)?
            .await;

        info!("getsessionURLs: {:?}", urls);

        if urls.is_empty() {
            return Err(ErrorCode::RendezVous_NotParticipatedGathering);
        }

        Ok(urls)
    }

    async fn update_session_host(
        &self,
        gid: u32,
        change_session_owner: bool,
    ) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let mut session = session.lock().await;

        if !session
            .connected_players
            .iter()
            .filter_map(|p| p.upgrade())
            .any(|p| p.base.pid == self.base.pid)
        {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        session.session.gathering.host_pid = self.base.pid;

        for player in &session.connected_players {
            let Some(player) = player.upgrade() else {
                continue;
            };

            player
                .remote
                .process_notification_event(NotificationEvent {
                    notif_type: 110_000,
                    pid_source: self.base.pid,
                    param_1: gid as PID,
                    param_2: self.base.pid,
                    #[cfg(feature = "third-notif-param")]
                    param_3: 0,
                    str_param: "".to_string(),
                })
                .await;
        }

        if change_session_owner {
            session.session.gathering.owner_pid = self.base.pid;

            for player in &session.connected_players {
                let Some(player) = player.upgrade() else {
                    continue;
                };

                player
                    .remote
                    .process_notification_event(NotificationEvent {
                        notif_type: 4000,
                        pid_source: self.base.pid,
                        param_1: gid as PID,
                        param_2: self.base.pid,
                        #[cfg(feature = "third-notif-param")]
                        param_3: 0,
                        str_param: "".to_string(),
                    })
                    .await;
            }
        }

        Ok(())
    }

    async fn migrate_gathering_ownership(
        &self,
        gid: u32,
        candidates: Vec<PID>,
        _participants_only: bool,
    ) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let mut session = session.lock().await;

        let candidate = candidates.get(0).ok_or(ErrorCode::Core_InvalidArgument)?;
        if session.session.gathering.owner_pid != self.base.pid {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        if !session
            .connected_players
            .iter()
            .filter_map(|p| p.upgrade())
            .any(|p| p.base.pid == *candidate)
        {
            return Err(ErrorCode::Core_InvalidArgument);
        }
        session.session.gathering.owner_pid = *candidate;

        for player in &session.connected_players {
            let Some(player) = player.upgrade() else {
                continue;
            };

            player
                .remote
                .process_notification_event(NotificationEvent {
                    notif_type: 4000,
                    pid_source: self.base.pid,
                    param_1: gid as PID,
                    param_2: *candidate as PID,
                    #[cfg(feature = "third-notif-param")]
                    param_3: 0,
                    str_param: "".to_string(),
                })
                .await;
        }

        Ok(())
    }

    async fn get_detailed_participants(
        &self,
        gid: u32,
    ) -> Result<Vec<ParticipantDetails>, ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let session = session.lock().await;

        let mut participant_details = Vec::with_capacity(session.connected_players.len());

        for player_weak in &session.connected_players {
            let Some(player) = player_weak.upgrade() else {
                continue;
            };

            participant_details.push(ParticipantDetails {
                participant: player.base.pid,
                name: "test".to_string(),
                message: String::new(),
                participants: 1,
            });
        }

        Ok(participant_details)
    }

    async fn update_session_host_v1(&self, gid: u32) -> Result<(), ErrorCode> {
        let should_migrate_owner = {
            let session = self.matchmake_manager.get_session(gid).await?;
            let session = session.lock().await;

            (session.session.gathering.flags & gathering_flags::PARTICIPANTS_CHANGE_OWNER) != 0
        };

        self.update_session_host(gid, should_migrate_owner).await
    }
}

impl MatchmakeExt for MatchmakeUser {
    async fn end_participation(&self, gid: u32, message: String) -> Result<bool, ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let mut session = session.lock().await;

        session
            .remove_player_from_session(self.base.pid, &message)
            .await?;

        Ok(true)
    }
}

impl NatTraversal for MatchmakeUser {
    async fn report_nat_properties(
        &self,
        nat_mapping: u32,
        nat_filtering: u32,
        _rtt: u32,
    ) -> Result<(), ErrorCode> {
        let mut urls = self.base.station_url.write().await;

        for station_url in urls.iter_mut() {
            station_url.options.retain(|o| match o {
                UrlOptions::NatMapping(_) | UrlOptions::NatFiltering(_) => false,
                _ => true,
            });

            station_url
                .options
                .push(UrlOptions::NatMapping(nat_mapping as u8));
            station_url
                .options
                .push(UrlOptions::NatFiltering(nat_filtering as u8));
        }

        Ok(())
    }

    async fn report_nat_traversal_result(
        &self,
        cid: u32,
        result: bool,
        rtt: u32,
    ) -> Result<(), ErrorCode> {
        info!(
            pid = self.base.pid,
            cid, result, rtt, "NAT traversal result"
        );
        Ok(())
    }

    #[cfg(feature = "nx")]
    async fn report_nat_traversal_result_detail(
        &self,
        cid: u32,
        result: bool,
        detail: i32,
        rtt: u32,
    ) -> Result<(), ErrorCode> {
        info!(
            pid = self.base.pid,
            cid, result, detail, rtt, "NAT traversal detail"
        );
        Ok(())
    }

    #[cfg(feature = "nx")]
    async fn request_probe_initiation(
        &self,
        target_list: Vec<StationUrl>,
    ) -> Result<(), ErrorCode> {
        let station_to_probe: String = {
            let stations = self.base.station_url.read().await;
            let station = stations
                .iter()
                .find(|station| {
                    station.options.iter().any(|option| {
                        matches!(option, UrlOptions::NatType(value) if value & PUBLIC != 0)
                    })
                })
                .or_else(|| stations.first())
                .ok_or(ErrorCode::Core_InvalidArgument)?;
            station.into()
        };
        self.request_probe_initialization_ext(target_list, station_to_probe)
            .await
    }

    #[cfg(not(feature = "nx"))]
    async fn request_probe_initiation(&self, _station_to_probe: String) -> Result<(), ErrorCode> {
        Ok(())
    }

    #[cfg(feature = "nx")]
    async fn get_relay_signature_key(
        &self,
    ) -> Result<
        (
            i32,
            rnex_rmc::util::date_time::DateTime,
            String,
            u16,
            i32,
            u32,
        ),
        ErrorCode,
    > {
        Ok((
            0,
            rnex_rmc::util::date_time::DateTime::now(),
            String::new(),
            0,
            0,
            0,
        ))
    }

    async fn request_probe_initialization_ext(
        &self,
        target_list: Vec<StationUrl>,
        station_to_probe: String,
    ) -> Result<(), ErrorCode> {
        info!(caller_pid = self.base.pid, targets = target_list.len(), station = %station_to_probe, "NAT probe requested");
        diagnostics::record("nat_probe_request", format_args!("caller_pid={} targets={target_list:?} station_to_probe={station_to_probe:?}", self.base.pid));
        let targets = {
            let users = self.matchmake_manager.users.read().await;
            let mut targets = Vec::with_capacity(target_list.len());

            for url in &target_list {
                let rvcid = url.options.iter().find_map(|option| match option {
                    UrlOptions::RVConnectionID(value) => Some(*value),
                    _ => None,
                });
                let target_pid = url.options.iter().find_map(|option| match option {
                    UrlOptions::PrincipalID(value) => Some(*value),
                    _ => None,
                });
                let user = rvcid
                    .and_then(|cid| users.get(&cid))
                    .and_then(|user| user.upgrade());

                let Some((rvcid, user)) = rvcid.zip(user).filter(|(cid, user)| {
                    target_pid == Some(user.base.pid)
                        && url.options.iter().all(|option| match option {
                            UrlOptions::RVConnectionID(value) => value == cid,
                            UrlOptions::PrincipalID(value) => *value == user.base.pid,
                            _ => true,
                        })
                }) else {
                    warn!(caller_pid = self.base.pid, ?rvcid, ?target_pid, "NAT probe target identity mismatch");
                    return Err(ErrorCode::RendezVous_InvalidPassword);
                };

                targets.push((rvcid, user));
            }

            targets
        };

        for (rvcid, user) in targets {
            let payload = station_to_probe
                .to_data()
                .map_err(|_| ErrorCode::Core_InvalidArgument)?;
            #[cfg(feature = "nx")]
            let call_id = 0xffff_0000 | u32::from(rnex_rmc::rand::random::<u16>());
            #[cfg(not(feature = "nx"))]
            let call_id = rnex_rmc::rand::random();
            let request = RMCMessage {
                protocol_id: 3,
                method_id: 2,
                call_id,
                rest_of_data: payload,
            };
            if let Err(error) = user
                .remote
                .get_connection()
                .make_raw_call_no_response(&request)
                .await
            {
                warn!(caller_pid = self.base.pid, target_pid = user.base.pid, rvcid, call_id, %error, "NAT InitiateProbe delivery failed");
                diagnostics::record("nat_probe_delivery_failed", format_args!("caller_pid={} target_pid={} rvcid={rvcid} call_id={call_id} error={error:?}", self.base.pid, user.base.pid));
                continue;
            }
            diagnostics::record("nat_probe_delivered", format_args!("caller_pid={} target_pid={} rvcid={rvcid} call_id={call_id} station_to_probe={station_to_probe:?}", self.base.pid, user.base.pid));
            info!(
                caller_pid = self.base.pid,
                target_pid = user.base.pid,
                rvcid,
                call_id,
                "NAT InitiateProbe queued for host"
            );
        }
        Ok(())
    }
}
