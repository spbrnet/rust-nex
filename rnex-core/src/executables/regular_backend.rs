use std::sync::{Arc, atomic::AtomicU32};

use crate::{
    executables::common::new_simple_backend,
    nex::{matchmake::MatchmakeManager, remote_console::RemoteConsole, user::User},
    rmc::protocols::RmcPureRemoteObject,
};

pub async fn start_regular_backend() {
    let mmm = Arc::new(MatchmakeManager {
        //gid_counter: AtomicU32::new(1),
        sessions: Default::default(),
        users: Default::default(),
        rv_cid_counter: AtomicU32::new(1),
    });

    let weak_mmm = Arc::downgrade(&mmm);

    MatchmakeManager::initialize_garbage_collect_thread(weak_mmm).await;

    new_simple_backend(move |c, r| {
        let mmm = mmm.clone();
        Arc::new_cyclic(move |this| User {
            this: this.clone(),
            ip: c.prudpsock_addr,
            pid: c.pid,
            remote: RemoteConsole::new(r),
            matchmake_manager: mmm,
            station_url: Default::default(),
        })
    })
    .await;
}
