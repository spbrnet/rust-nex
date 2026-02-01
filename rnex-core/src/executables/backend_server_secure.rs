use cfg_if::cfg_if;
use rnex_core::common::setup;
use rnex_core::executables::common::new_simple_backend;
use rnex_core::executables::friends_backend::start_friends_backend;
use rnex_core::nex::matchmake::MatchmakeManager;
use rnex_core::nex::remote_console::RemoteConsole;
use rnex_core::nex::user::User;
use rnex_core::rmc::protocols::{RemoteDisconnectable, RmcPureRemoteObject};
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

#[tokio::main]
async fn main() {
    setup();

    cfg_if! {
        if #[cfg(feature = "friends")]{
            start_friends_backend().await;
        } else {
            regular_backend::start_regular_backend().await
        }
    }
}
