use crate::define_rmc_proto;
use crate::nex::common::get_station_urls;
use crate::nex::matchmake::{ExtendedMatchmakeSession, MatchmakeManager};
use crate::nex::remote_console::RemoteConsole;
use crate::rmc::protocols::matchmake::{
    Matchmake, RawMatchmake, RawMatchmakeInfo, RemoteMatchmake,
};
use crate::rmc::protocols::nat_traversal::{
    NatTraversal, RawNatTraversal, RawNatTraversalInfo, RemoteNatTraversal,
    RemoteNatTraversalConsole,
};
use rnex_core::PID;
use rnex_core::kerberos::KerberosDateTime;
use rnex_core::prudp::station_url::StationUrl;
use rnex_core::prudp::station_url::UrlOptions::{
    Address, NatFiltering, NatMapping, Port, RVConnectionID,
};
use rnex_core::rmc::protocols::matchmake_ext::{
    MatchmakeExt, RawMatchmakeExt, RawMatchmakeExtInfo, RemoteMatchmakeExt,
};
use rnex_core::rmc::protocols::matchmake_extension::{
    MatchmakeExtension, RawMatchmakeExtension, RawMatchmakeExtensionInfo, RemoteMatchmakeExtension,
};
use rnex_core::rmc::protocols::ranking::{Ranking, RawRanking, RawRankingInfo, RemoteRanking};
use rnex_core::rmc::protocols::secure::{RawSecure, RawSecureInfo, RemoteSecure, Secure};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::any::Any;
use rnex_core::rmc::structures::matchmake::{
    AutoMatchmakeParam, CreateMatchmakeSessionParam, JoinMatchmakeSessionParam, MatchmakeSession,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;

use crate::rmc::protocols::notifications::{NotificationEvent, RemoteNotification};
use log::{error, info};
use macros::rmc_struct;
use rnex_core::prudp::socket_addr::PRUDPSockAddr;
use rnex_core::rmc::protocols::ranking::{
    CompetitionRankingGetParam, CompetitionRankingScoreData, CompetitionRankingScoreInfo,
};
use rnex_core::rmc::response::ErrorCode::{Core_InvalidArgument, RendezVous_AccountExpired};
use rnex_core::rmc::structures::qbuffer::QBuffer;
use rnex_core::rmc::structures::qresult::QResult;
use rnex_core::rmc::structures::ranking::UploadCompetitionData;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, RwLock};

define_rmc_proto!(
    proto UserProtocol{
        Secure,
        MatchmakeExtension,
        MatchmakeExt,
        Matchmake,
        NatTraversal,
        Ranking
    }
);

#[rmc_struct(UserProtocol)]
pub struct User {
    pub pid: PID,
    pub ip: PRUDPSockAddr,
    pub this: Weak<User>,
    pub remote: RemoteConsole,
    pub station_url: RwLock<Vec<StationUrl>>,
    pub matchmake_manager: Arc<MatchmakeManager>,
}

impl Secure for User {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.matchmake_manager.next_cid();

        println!("{:?}", station_urls);

        let mut users = self.matchmake_manager.users.write().await;
        users.insert(cid, self.this.clone());
        drop(users);

        let stations = get_station_urls(&station_urls, self.ip, self.pid, cid).await?;

        let first = stations.first().unwrap().clone();

        let mut lock = self.station_url.write().await;

        *lock = stations;

        drop(lock);

        let result = QResult::success(ErrorCode::Core_Unknown);

        Ok((result, cid, first))
    }

    async fn register_ex(
        &self,
        station_urls: Vec<StationUrl>,
        _data: Any,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        self.register(station_urls).await
    }

    async fn replace_url(&self, target_url: StationUrl, dest: StationUrl) -> Result<(), ErrorCode> {
        let mut lock = self.station_url.write().await;

        let Some(target_addr) = target_url.options.iter().find(|v| matches!(v, Address(_))) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(target_port) = target_url.options.iter().find(|v| matches!(v, Port(_))) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(replacement_target) = lock.iter_mut().find(|url| {
            url.options.iter().any(|o| o == target_addr)
                && url.options.iter().any(|o| o == target_port)
        }) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };
        *replacement_target = dest;

        drop(lock);

        Ok(())
    }
}

impl MatchmakeExtension for User {
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

    async fn get_playing_session(&self, _pids: Vec<u32>) -> Result<Vec<()>, ErrorCode> {
        Ok(Vec::new())
    }

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
        println!("{:?}", create_session_param);

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
                        .any(|p| *p == u.pid)
                });
            for player in players {
                joining_players.push(Arc::downgrade(&player));
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
        if join_session_param.user_password != session.session.user_password {
            return Err(ErrorCode::RendezVous_InvalidPassword);
        }

        session
            .connected_players
            .retain(|v| v.upgrade().is_some_and(|v| v.pid != self.pid));

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
                        .any(|p| *p == u.pid)
                });
            for player in players {
                joining_players.push(Arc::downgrade(&player));
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
        println!("{:?}", param);

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
                .filter(|u| param.additional_participants.iter().any(|p| *p == u.pid));
            for player in players {
                joining_players.push(Arc::downgrade(&player));
            }
        }

        drop(users);

        let sessions = self.matchmake_manager.sessions.read().await;
        for session in sessions.values() {
            let mut session = session.lock().await;

            println!("checking session!");

            if !session.is_joinable() {
                continue;
            }

            let mut bool_matched_criteria = false;

            for criteria in &param.search_criteria {
                if session.matches_criteria(criteria)? {
                    bool_matched_criteria = true;
                }
            }

            if bool_matched_criteria {
                session
                    .add_players(&joining_players, param.join_message)
                    .await;

                return Ok(session.session.clone());
            }
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

        session.session.attributes[attrib_index as usize] = attrib_val;

        Ok(())
    }
}

impl Matchmake for User {
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
            .filter(|u| u.pid == session.session.gathering.host_pid)
            .map(|u| async move { u.station_url.read().await.clone() })
            .next()
            .ok_or(ErrorCode::RendezVous_SessionClosed)?
            .await;

        println!("{:?}", urls);

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

        session.session.gathering.host_pid = self.pid;

        for player in &session.connected_players {
            let Some(player) = player.upgrade() else {
                continue;
            };

            player
                .remote
                .process_notification_event(NotificationEvent {
                    notif_type: 110000,
                    pid_source: self.pid,
                    param_1: gid as PID,
                    param_2: self.pid,
                    param_3: 0,
                    str_param: "".to_string(),
                })
                .await;
        }

        if change_session_owner {
            session.session.gathering.owner_pid = self.pid;

            for player in &session.connected_players {
                let Some(player) = player.upgrade() else {
                    continue;
                };

                player
                    .remote
                    .process_notification_event(NotificationEvent {
                        notif_type: 4000,
                        pid_source: self.pid,
                        param_1: gid as PID,
                        param_2: self.pid,
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

        let candidate = candidates.get(0).ok_or(Core_InvalidArgument)?;

        session.session.gathering.owner_pid = *candidate;

        for player in &session.connected_players {
            let Some(player) = player.upgrade() else {
                continue;
            };

            player
                .remote
                .process_notification_event(NotificationEvent {
                    notif_type: 4000,
                    pid_source: self.pid,
                    param_1: gid as PID,
                    param_2: *candidate as PID,
                    param_3: 0,
                    str_param: "".to_string(),
                })
                .await;
        }

        Ok(())
    }
}

impl MatchmakeExt for User {
    async fn end_participation(&self, gid: u32, message: String) -> Result<bool, ErrorCode> {
        let session = self.matchmake_manager.get_session(gid).await?;
        let mut session = session.lock().await;

        session
            .remove_player_from_session(self.pid, &message)
            .await?;

        Ok(true)
    }
}

impl NatTraversal for User {
    async fn report_nat_properties(
        &self,
        nat_mapping: u32,
        nat_filtering: u32,
        _rtt: u32,
    ) -> Result<(), ErrorCode> {
        let mut urls = self.station_url.write().await;

        for station_url in urls.iter_mut() {
            station_url.options.retain(|o| match o {
                NatMapping(_) | NatFiltering(_) => false,
                _ => true,
            });

            station_url.options.push(NatMapping(nat_mapping as u8));
            station_url.options.push(NatFiltering(nat_filtering as u8));
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
        Err(RendezVous_AccountExpired)
    }

    async fn request_probe_initialization_ext(
        &self,
        target_list: Vec<String>,
        station_to_probe: String,
    ) -> Result<(), ErrorCode> {
        let users = self.matchmake_manager.users.read().await;

        println!(
            "requesting station probe for {:?} to {:?}",
            target_list, station_to_probe
        );

        for target in target_list {
            let Ok(url) = StationUrl::try_from(target.as_ref()) else {
                continue;
            };

            let Some(RVConnectionID(v)) = url
                .options
                .into_iter()
                .find(|o| matches!(o, &RVConnectionID(_)))
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

#[derive(Serialize, Deserialize)]
pub struct CompetitionPostResults {
    pub splatfest_id: u32,
    pub score: u32,
    pub team_id: u8,
    pub team_win: u8,
    pub user: PID,
}

// Seperate function because I cannot give a fuck right now
fn fetch_team_votes(fest_id: u32) -> Result<Vec<u32>, ErrorCode> {
    let endpoint_votes = env::var("RNEX_SPLATOON_RESULTS_VOTES_GET").map_err(|_| {
        error!("RNEX_SPLATOON_RESULTS_VOTES_GET not set");
        ErrorCode::RendezVous_InvalidConfiguration
    })?;

    let url_votes = format!("{}?splatfest_id={}", endpoint_votes, fest_id);
    let mut response = ureq::get(&url_votes).call().map_err(|e| {
        error!("GET for votes failed: {:?}", e);
        ErrorCode::RendezVous_InvalidConfiguration
    })?;

    let body = response.body_mut().read_to_string().map_err(|e| {
        error!("failed to read votes body: {:?}", e);
        ErrorCode::RendezVous_InvalidConfiguration
    })?;

    let body = body.trim().trim_start_matches('[').trim_end_matches(']');
    let votes: Result<Vec<u32>, _> = body.split(',').map(|s| u32::from_str(s.trim())).collect();

    votes.map_err(|e| {
        error!("failed to parse votes: {:?}", e);
        ErrorCode::RendezVous_InvalidConfiguration
    })
}

impl Ranking for User {
    async fn competition_ranking_get_param(
        &self,
        param: CompetitionRankingGetParam,
    ) -> Result<Vec<CompetitionRankingScoreInfo>, ErrorCode> {
        let fest_id = param.festival_ids.get(0).copied().unwrap_or(0);

        let endpoint_results = env::var("RNEX_SPLATOON_RESULTS_GET").map_err(|_| {
            error!("RNEX_SPLATOON_RESULTS_GET not set");
            ErrorCode::RendezVous_InvalidConfiguration
        })?;

        let url_results = format!("{}?splatfest_id={}", endpoint_results, fest_id);
        let response_results = ureq::get(&url_results).call();

        let results: Vec<CompetitionPostResults> = match response_results {
            Ok(mut res) => res.body_mut().read_json().map_err(|e| {
                error!("failed to parse JSON: {:?}", e);
                ErrorCode::RendezVous_InvalidConfiguration
            })?,
            Err(e) => {
                error!("GET failed: {:?}", e);
                return Err(ErrorCode::RendezVous_InvalidConfiguration);
            }
        };

        let team_votes = fetch_team_votes(fest_id)?;
        let mut wins = vec![0u32, 0u32];
        for r in &results {
            let won_team = (r.team_id ^ (!r.team_win)) & 1;
            if let Some(team) = wins.get_mut(won_team as usize) {
                *team += 1
            };
        }

        let score_data: Vec<CompetitionRankingScoreData> = results
            .iter()
            .map(|r| CompetitionRankingScoreData {
                unk: 1,
                pid: r.user,
                score: r.score,
                modified: KerberosDateTime::now(),
                unk2: 1,
                appdata: QBuffer(vec![]),
            })
            .collect();

        let info = CompetitionRankingScoreInfo {
            fest_id,
            score_data,
            unk: 0,
            team_wins: wins,
            team_votes,
        };

        Ok(vec![info])
    }

    async fn upload_competition_ranking_score(
        &self,
        param: UploadCompetitionData,
    ) -> Result<bool, ErrorCode> {
        info!("fest results for user {:?}:", self.pid);
        info!("fest id: {:?}", param.splatfest_id);
        info!("score: {:?}", param.score);
        info!("team id: {:?}", param.team_id);
        info!("did current team win: {:?}", param.team_win);

        let endpoint = match env::var("RNEX_SPLATOON_RESULTS_POST") {
            Ok(url) => url,
            Err(_) => {
                error!("RNEX_SPLATOON_RESULTS_POST not set");
                return Ok(false);
            }
        };

        let payload = CompetitionPostResults {
            splatfest_id: param.splatfest_id,
            score: param.score,
            team_id: param.team_id,
            team_win: param.team_win,
            user: self.pid,
        };

        let json_body = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                error!("error making json_body: {:?}", e);
                return Ok(false);
            }
        };

        let response = ureq::post(&endpoint)
            .header("Content-Type", "application/json")
            .send(json_body);

        match response {
            Ok(res) => {
                info!("POST worked: {}", res.status());
            }
            Err(e) => {
                error!("POST borked: {:?}", e);
            }
        }

        Ok(true)
    }
}
