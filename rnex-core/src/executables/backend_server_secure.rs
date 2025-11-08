use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use rnex_core::common::setup;
use rnex_core::executables::common::{new_simple_backend};
use rnex_core::nex::matchmake::MatchmakeManager;
use rnex_core::nex::remote_console::RemoteConsole;
use rnex_core::nex::user::User;
use rnex_core::rmc::protocols::RemoteInstantiatable;

#[tokio::main]
async fn main() {
    setup();


    let mmm = Arc::new(MatchmakeManager{
        gid_counter: AtomicU32::new(1),
        sessions: Default::default(),
        users: Default::default(),
        rv_cid_counter: AtomicU32::new(1),
    });

    let weak_mmm = Arc::downgrade(&mmm);

    MatchmakeManager::initialize_garbage_collect_thread(weak_mmm).await;

    new_simple_backend(move |c, r|{
        let mmm = mmm.clone();
        Arc::new_cyclic(move |this| User{
            this: this.clone(),
            ip: c.prudpsock_addr,
            pid:c.pid,
            remote: RemoteConsole::new(r),
            matchmake_manager: mmm,
            station_url: Default::default()
        })
    }).await;
}