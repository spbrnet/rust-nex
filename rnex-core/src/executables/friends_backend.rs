use std::sync::{Arc, atomic::AtomicU32};

use crate::{
    executables::common::new_simple_backend,
    nex::friends_handler::{FriendsManager, FriendsUser},
};

pub async fn start_friends_backend() {
    let fm = Arc::new(FriendsManager {
        cid_counter: AtomicU32::new(1),
    });

    new_simple_backend(move |c, r| {
        let fm = fm.clone();
        Arc::new_cyclic(move |this| FriendsUser {
            fm,
            addr: c.prudpsock_addr,
            pid: c.pid,
        })
    })
    .await;
}
