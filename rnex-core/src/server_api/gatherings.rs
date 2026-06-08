use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use rnex_server_api::gatherings::{
    Gatherings, gathering_info_service_server::GatheringInfoService,
};
use tonic::{Request, Response, Status, async_trait};

use crate::{
    nex::matchmake::{ExtendedMatchmakeSession, MatchmakeManager},
    rmc::structures::matchmake::{self, MatchmakeSession},
    server_api,
};

impl Into<rnex_server_api::gatherings::Gathering> for &ExtendedMatchmakeSession {
    fn into(self) -> rnex_server_api::gatherings::Gathering {
        let players = self.get_active_players().map(|p| *p.pid as u64).collect();
        let ExtendedMatchmakeSession {
            session:
                MatchmakeSession {
                    gathering:
                        matchmake::Gathering {
                            state,
                            flags,
                            host_pid,
                            description,
                            maximum_participants,
                            minimum_participants,
                            owner_pid,
                            participant_policy,
                            policy_argument,
                            self_gid,
                        },
                    application_buffer,
                    attributes,
                    gamemode,
                    matchmake_system_type,
                    open_participation,
                    participation_count,
                    session_key,
                },
            connected_players,
        } = self;
        rnex_server_api::gatherings::Gathering {
            players,
            description: description.clone(),
            flags: *flags,
            host_pid: *host_pid as _,
            maximum_participants: *maximum_participants as _,
            minimum_participants: *minimum_participants as _,
            owner_pid: *owner_pid as _,
            participant_policy: *participant_policy,
            policy_argument: *policy_argument,
            self_gid: *self_gid,
            state: *state,
            application_buffer: application_buffer.clone(),
            attributes: attributes.clone(),
            ..Default::default()
        }
    }
}

struct GatheringsApi(Arc<MatchmakeManager>);

#[async_trait]
impl GatheringInfoService for GatheringsApi {
    async fn get_gatherings(
        &self,
        request: Request<()>,
    ) -> std::result::Result<Response<Gatherings>, Status> {
        Ok(Response::new(Gatherings {
            gatherings: self
                .0
                .sessions
                .read()
                .await
                .keys()
                .map(|gid| {
                    let mut hasher = DefaultHasher::new();
                    gid.hash(&mut hasher);
                    hasher.finish()
                })
                .collect(),
        }))
    }
}
