use std::sync::Arc;
use async_trait::async_trait;
use rocket::{get, routes, Request, State};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use tokio::task::JoinHandle;
use crate::nex::matchmake::MatchmakeManager;
use crate::rmc::protocols::notifications::NotificationEvent;

struct RnexApiAuth;

#[async_trait]
impl<'r> FromRequest<'r> for RnexApiAuth{

    type Error = ();
    async fn from_request<'a>(request: &'r Request<'a>) -> Outcome<Self, Self::Error> {
        Outcome::Success(RnexApiAuth)
    }
}


#[get("/gatherings")]
async fn gatherings(mmm: &State<Arc<MatchmakeManager>>) -> Json<Vec<u32>>{
    let matches = mmm.sessions.read().await;

    Json(matches.keys().map(|v| *v).collect())
}

#[get("/gathering/<gid>/players")]
async fn players_in_match(mmm: &State<Arc<MatchmakeManager>>, gid: u32) -> Option<Json<Vec<u32>>>{
    let mmm = mmm.sessions.read().await;

    let gathering = mmm.get(&gid)?;

    let gathering = gathering.clone();

    drop(mmm);

    let gathering = gathering.lock().await;

    Some(Json(gathering.connected_players.iter().filter_map(|p| p.upgrade()).map(|p| p.pid).collect()))
}
/*
#[get("/player/<pid>/disconnect")]
async fn disconnect_player(_auth: RnexApiAuth, mmm: &State<Arc<MatchmakeManager>>, pid: u32) -> Option<()>{
    // this doesnt work and is broken, there might be some other way to remotely close gatherings...
    // also if anyone gets this working change it to POST cause the only reason its get is because
    // that makes testing it easier
    let mmm = mmm.users.read().await;

    for player in mmm.values().filter_map(|p| p.upgrade()).filter(|p| p.pid == pid) {
        player.remote.get_connection().0.close_connection().await;
    }   


    Some(())
}*/

#[get("/gathering/<gid>/close")]
async fn close_gathering(_auth: RnexApiAuth, mmm: &State<Arc<MatchmakeManager>>, gid: u32) -> Option<()>{
    // this doesnt work and is broken, there might be some other way to remotely close gatherings...
    // also if anyone gets this working change it to POST cause the only reason its get is because
    // that makes testing it easier
    let mmm = mmm.sessions.read().await;

    let gathering = mmm.get(&gid)?;

    let gathering = gathering.clone();

    drop(mmm);

    let gathering = gathering.lock().await;

    gathering.broadcast_notification(&NotificationEvent{
        pid_source: gathering.session.gathering.owner_pid,
        notif_type: 109000,
        param_1: gathering.session.gathering.self_gid,
        ..Default::default()
    }).await;

    Some(())
}

pub async fn start_web(mgr: Arc<MatchmakeManager>) -> JoinHandle<()> {
    tokio::spawn(async move {
        rocket::build()
            .mount("/", routes![gatherings, players_in_match, close_gathering])
            .manage(mgr)
            .launch().await
            .expect("unable to start webserver");
    })
}