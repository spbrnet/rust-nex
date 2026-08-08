use std::sync::Arc;

use rnex_base::user::BaseUser;
use rnex_base_protos::ResultsRange;
use rnex_mm_protos::{
    LocalMatchMakingProtocol, RemoteMatchMakingClientProtocol,
    matchmake::{
        AutoMatchmakeParam, CreateMatchmakeSessionParam, Gathering, JoinMatchmakeSessionParam,
        Matchmake, MatchmakeSession, MatchmakeSessionSearchCriteria,
    },
    matchmake_ext::MatchmakeExt,
    matchmake_extension::MatchmakeExtension,
    nat_traversal::NatTraversal,
    nat_traversal::RemoteNatTraversalConsole,
    notifications::{
        NotificationEvent, RemoteNotification,
        notification_types::{END_GATHERING, REQUEST_JOIN_GATHERING},
    },
};
use rnex_rmc::{any::Any, response::ErrorCode, rmc_struct};
use rnex_server::{PassthroughInitModule, WeakPassthroughInitModule};
use rnex_util::{
    PID,
    station_url::{StationUrl, UrlOptions},
};
use tokio::sync::Mutex;
use tracing::info;

use crate::matchmake::{ExtendedMatchmakeSession, MatchmakeManager};

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

        session.session.open_participation = false;

        Ok(())
    }

    async fn open_participation(&self, gid: u32) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;

        session.session.open_participation = true;

        Ok(())
    }

    async fn browse_matchmake_session(
        &self,
        browse_criteria: MatchmakeSessionSearchCriteria,
        result_range: ResultsRange,
    ) -> Result<Vec<Any<Gathering>>, ErrorCode> {
        let results = self
            .matchmake_manager
            .search_by_criteria(&[browse_criteria])
            .await?;
        let mm_list = result_range.make_from_list(&results[..]);

        let mut list = Vec::with_capacity(mm_list.len());

        for mm_sess in mm_list {
            let mm_sess = mm_sess.lock().await;
            list.push(Any::new(&mm_sess.session).expect("type error"));
        }

        Ok(list)
    }

    async fn get_playing_session(&self, _pids: Vec<u32>) -> Result<Vec<()>, ErrorCode> {
        Ok(Vec::new())
    }

    #[cfg(feature = "v3-5-0")]
    async fn update_progress_score(&self, gid: u32, progress: u8) -> Result<(), ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;

        let mut session = session.lock().await;

        session.session.progress_score = progress;

        Ok(())
    }

    async fn create_matchmake_session_with_param(
        &self,
        create_session_param: CreateMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        info!("session paramater: {:?}", create_session_param);

        let gid = self.matchmake_manager.next_gid();

        let mut new_session = ExtendedMatchmakeSession::from_matchmake_session(
            gid,
            create_session_param.matchmake_session,
            &self.this.clone(),
        )
        .await;

        let mut joining_players = vec![self.this.clone()];

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(create_session_param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;

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
            .await;

        let session = new_session.session.clone();

        let mut sessions = self.matchmake_manager.sessions.write().await;
        sessions.insert(gid, Arc::new(Mutex::new(new_session)));
        drop(sessions);

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

        let mut session = session.lock().await;

        #[cfg(feature = "v3-5-0")]
        {
            if join_session_param.user_password != session.session.user_password {
                return Err(ErrorCode::RendezVous_MatchmakeSessionUserPasswordUnmatch);
            }
        }

        session
            .connected_players
            .retain(|v| v.upgrade().is_some_and(|v| v.base.pid != self.base.pid));

        let mut joining_players = vec![self.this.clone()];

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(join_session_param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;

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

        session
            .add_players(&joining_players, join_session_param.join_message)
            .await;

        let mm_session = session.session.clone();

        Ok(mm_session)
    }

    async fn auto_matchmake_with_param_postpone(
        &self,
        param: AutoMatchmakeParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        info!("autommparam: {:?}", param);

        let mut joining_players = vec![self.this.clone()];

        let users = self.matchmake_manager.users.read().await;

        if let Ok(old_gathering) = self
            .matchmake_manager
            .get_session(param.gid_for_participation_check)
            .await
        {
            let old_gathering = old_gathering.lock().await;

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

        let sessions = self
            .matchmake_manager
            .search_by_criteria(&param.search_criteria[..])
            .await?;

        if let Some(session) = sessions.get(0) {
            let mut session = session.lock().await;
            session
                .add_players(&joining_players, param.join_message)
                .await;

            return Ok(session.session.clone());
        }

        drop(sessions);

        info!("making new session!");

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

        session.session.attributes[attrib_index as usize] = attrib_val;

        Ok(())
    }

    async fn create_matchmake_session(
        &self,
        gathering: Any<Gathering>,
        message: String,
    ) -> Result<(u32, Vec<u8>), ErrorCode> {
        info!("gathering: {:?}", gathering);
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

        if session.session.gathering.host_pid == self.base.pid {
            return Err(ErrorCode::RendezVous_PermissionDenied);
        }
        if session.session.gathering.owner_pid == self.base.pid {
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
        sess.add_players(&[self.this.clone()], message).await;

        Ok(sess.session.session_key.clone())
    }

    async fn auto_matchmake_with_search_criteria_postpone(
        &self,
        criteria: Vec<MatchmakeSessionSearchCriteria>,
        gathering: Any<Gathering>,
        join_message: String,
    ) -> Result<Any<Gathering>, ErrorCode> {
        let session: MatchmakeSession = gathering.try_get_as()?;

        info!("automm criteria: {:?}", criteria);

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
        _cid: u32,
        _result: bool,
        _rtt: u32,
    ) -> Result<(), ErrorCode> {
        Ok(())
    }

    async fn request_probe_initiation(&self, _station_to_probe: String) -> Result<(), ErrorCode> {
        info!("NO!");
        Err(ErrorCode::RendezVous_AccountExpired)
    }

    async fn request_probe_initialization_ext(
        &self,
        target_list: Vec<StationUrl>,
        station_to_probe: String,
    ) -> Result<(), ErrorCode> {
        let users = self.matchmake_manager.users.read().await;

        info!(
            "requesting station probe for {:?} to {:?}",
            target_list, station_to_probe
        );

        for url in target_list {
            let Some(UrlOptions::RVConnectionID(v)) = url
                .options
                .into_iter()
                .find(|o| matches!(o, &UrlOptions::RVConnectionID(_)))
            else {
                continue;
            };

            let Some(v) = users.get(&v) else {
                continue;
            };

            let Some(user) = v.upgrade() else {
                continue;
            };

            user.remote
                .request_probe_initiation(station_to_probe.clone())
                .await;
        }

        info!("finished probing");

        Ok(())
    }
}
