use std::{net::SocketAddr, str::FromStr};

use rnex_server_api::meta::server_meta_service_server::ServerMetaServiceServer;
use tonic::transport::Server;

#[cfg(not(feature = "friends"))]
use crate::nex::matchmake::MatchmakeManager;
use crate::server_api::meta::ServerMeta;
#[cfg(not(feature = "friends"))]
use std::sync::Arc;
#[cfg(not(feature = "friends"))]
mod gatherings;

mod meta;

pub async fn launch_server_api(#[cfg(not(feature = "friends"))] mmm: Arc<MatchmakeManager>) {
    let mut server = Server::builder().add_service(ServerMetaServiceServer::new(ServerMeta));

    server
        .serve(SocketAddr::from_str("0.0.0.0:80").expect("unable to make sockaddr from ip"))
        .await;
}
