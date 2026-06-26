use std::sync::{Arc, atomic::AtomicU32};

use tokio::sync::{Mutex, mpsc::channel};

use crate::{
    executables::common::new_simple_backend,
    nex::{
        matchmake::MatchmakeManager,
        remote_console::RemoteConsole,
        user::{ConnectionTicket, User},
    },
    rmc::protocols::RmcPureRemoteObject,
};

pub async fn start_regular_backend() {
    let mmm = Arc::new(MatchmakeManager {
        //gid_counter: AtomicU32::new(1),
        sessions: Default::default(),
        users: Default::default(),
        users_by_pid: Default::default(),
        rv_cid_counter: AtomicU32::new(1),
    });

    let weak_mmm = Arc::downgrade(&mmm);

    MatchmakeManager::initialize_garbage_collect_thread(weak_mmm).await;

    new_simple_backend(move |c, r| {
        let mmm = mmm.clone();
        Arc::new_cyclic(move |this| {
            let (join_tickets_stage1_sender, join_tickets_stage1_recv) =
                channel::<ConnectionTicket>(100);
            let join_tickets_stage1_recv = Mutex::new(join_tickets_stage1_recv);

            let (join_tickets_stage2_sender, join_tickets_stage2_recv) =
                channel::<ConnectionTicket>(100);
            let join_tickets_stage2_recv = Mutex::new(join_tickets_stage2_recv);
            let cid = mmm.next_cid();

            User {
                cid,
                this: this.clone(),
                ip: c.prudpsock_addr,
                pid: c.pid,
                remote: RemoteConsole::new(r),
                matchmake_manager: mmm,
                station_url: Default::default(),
                join_tickets_stage1_recv,
                join_tickets_stage1_sender,
                join_tickets_stage2_recv,
                join_tickets_stage2_sender,
                self_join_ticket_requesters: Default::default(),
                remote_join_ticket_requesters: Default::default(),
            }
        })
    })
    .await;
}
