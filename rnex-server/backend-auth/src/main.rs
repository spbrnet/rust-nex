use rnex_auth::AuthModule;
use rnex_server::{ConnectionInitData, launch_rnex_module_server};
#[tokio::main]
async fn main() {
    launch_rnex_module_server! {
        ConnectionInitData;
        AuthModule
    }
}
