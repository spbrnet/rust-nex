mod admin;
use std::net::SocketAddr;
use rnex_base::BaseModule;
#[cfg(feature = "datastore")]
use rnex_ds::DatastoreModule;
#[cfg(feature = "friends")]
use rnex_fpd::FriendsModule;
#[cfg(feature = "match-making")]
use rnex_mm::MatchMakeModule;
#[cfg(feature = "messaging")]
use rnex_msg::MessagingModule;
#[cfg(feature = "ranking")]
use rnex_rk::RankingModule;
use rnex_server::{ConnectionInitData, connection_registry, launch_rnex_module_server};

fn admin_addr() -> SocketAddr {
    std::env::var("RNEX_ADMIN_GRPC_ADDR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 50051)))
}

#[tokio::main]
async fn main() {
    let registry = connection_registry();
    let token = std::env::var("RNEX_ADMIN_GRPC_TOKEN").ok();

    let admin_task = tokio::spawn(async move {
        let result: Result<(), tonic::transport::Error> =
            admin::serve(admin_addr(), registry, token).await;

        if let Err(err) = result {
            rnex_server::tracing::error!(%err, "admin gRPC server stopped");
        }
    });

    launch_rnex_module_server! {
        ConnectionInitData;
        BaseModule,
        #[cfg(feature="match-making")]
        MatchMakeModule,
        #[cfg(feature="datastore")]
        DatastoreModule,
        #[cfg(feature="friends")]
        FriendsModule,
        #[cfg(feature="ranking")]
        RankingModule,
        #[cfg(feature="messaging")]
        MessagingModule
    }

    admin_task.abort();
    let _ = admin_task.await;
}