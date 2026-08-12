mod admin;

use std::net::SocketAddr;

use rnex_auth::AuthModule;
use rnex_server::{ConnectionInitData, launch_rnex_module_server};

fn admin_addr() -> SocketAddr {
    std::env::var("RNEX_AUTH_ADMIN_GRPC_ADDR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 50052)))
}

#[tokio::main]
async fn main() {
    let token = std::env::var("RNEX_AUTH_ADMIN_GRPC_TOKEN").ok();

    let admin_task = tokio::spawn(async move {
        if let Err(err) = admin::serve(admin_addr(), token).await {
            rnex_server::tracing::error!(%err, "auth admin gRPC server stopped");
        }
    });

    launch_rnex_module_server! {
        ConnectionInitData;
        AuthModule
    }

    admin_task.abort();
    let _ = admin_task.await;
}