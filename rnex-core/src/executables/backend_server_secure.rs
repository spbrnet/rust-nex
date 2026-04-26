use cfg_if::cfg_if;
use rnex_core::common::setup;

#[tokio::main]
async fn main() {
    setup();

    cfg_if! {
        if #[cfg(feature = "friends")]{
            use rnex_core::executables::friends_backend::start_friends_backend;
            start_friends_backend().await;
        } else {
            use rnex_core::executables::regular_backend;
            regular_backend::start_regular_backend().await
        }
    }
}
