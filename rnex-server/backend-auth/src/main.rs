use rnex_auth::AuthModule;
use rnex_server::launch_rnex_module_server;
use std::net::SocketAddr;
#[tokio::main]
async fn main() {
    launch_rnex_module_server! {
        SocketAddr;
        AuthModule
    }
}
