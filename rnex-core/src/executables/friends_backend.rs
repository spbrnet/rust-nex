use std::{
    io::Cursor,
    net::SocketAddrV4,
    sync::{Arc, atomic::AtomicU32},
};

use log::error;
use tokio::net::TcpListener;

use crate::{
    executables::common::{OWN_IP_PRIVATE, SERVER_PORT},
    nex::friends_handler::{FriendsGuest, FriendsManager, FriendsUser, RemoteFriendRemote},
    reggie::UnitPacketRead,
    rmc::{
        protocols::{RmcPureRemoteObject, new_rmc_gateway_connection},
        structures::RmcSerialize,
    },
    rnex_proxy_common::ConnectionInitData,
};

pub async fn start_friends_backend() {
    let fm = Arc::new(FriendsManager {
        cid_counter: AtomicU32::new(1),
        users: Default::default(),
    });
    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
        .await
        .unwrap();
    while let Ok((mut stream, _addr)) = listen.accept().await {
        let buffer = match stream.read_buffer().await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "an error ocurred whilst reading connection data buffer: {:?}",
                    e
                );
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let c = match user_connection_data {
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilst reading connection data: {:?}", e);
                continue;
            }
        };
        let fm = fm.clone();
        if c.pid != 100 {
            new_rmc_gateway_connection(stream.into(), move |r| {
                Arc::new_cyclic(move |this| FriendsUser {
                    fm,
                    addr: c.prudpsock_addr,
                    pid: c.pid,
                    data: Default::default(),
                    current_friends: Default::default(),
                    this: this.clone(),
                    remote: RemoteFriendRemote::new(r),
                })
            });
        } else {
            new_rmc_gateway_connection(stream.into(), move |_| {
                Arc::new_cyclic(move |_| FriendsGuest {
                    fm,
                    addr: c.prudpsock_addr,
                })
            });
        }
    }
}
