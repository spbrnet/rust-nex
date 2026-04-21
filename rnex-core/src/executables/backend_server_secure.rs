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
        } else if #[cfg(feature = "datastore")] {
            use rnex_core::executables::common::DB_POOL;
            use sqlx::PgPool;
            let database_url = std::env::var("RNEX_DATASTORE_DATABASE_URL")
                .expect("RNEX_DATASTORE_DATABASE_URL must be set");

            let pool = PgPool::connect(&database_url)
                .await
                .expect("Failed to create pool");

            DB_POOL.set(pool).expect("failed to set global DB_POOL");
            use rnex_core::executables::regular_backend;
            regular_backend::start_regular_backend().await
        } else {
            use rnex_core::executables::regular_backend;
            regular_backend::start_regular_backend().await
        }
    }
}
