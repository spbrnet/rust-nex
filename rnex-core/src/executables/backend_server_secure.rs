use std::io::Cursor;
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::reggie::UnitPacketRead;
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use log::{error, info};
use tokio::net::TcpListener;
use tokio::task;
use rnex_core::common::setup;
use rnex_core::executables::common::{OWN_IP_PRIVATE, SERVER_PORT};
use rnex_core::nex::matchmake::MatchmakeManager;
use rnex_core::nex::remote_console::RemoteConsole;
use rnex_core::nex::user::User;
use rnex_core::rmc::protocols::new_rmc_gateway_connection;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use rnex_core::rmc::protocols::RemoteInstantiatable;

#[tokio::main]
async fn main() {
    setup();

    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT)).await.unwrap();

    let mmm = Arc::new(MatchmakeManager{
        gid_counter: AtomicU32::new(1),
        sessions: Default::default(),
        users: Default::default(),
        rv_cid_counter: AtomicU32::new(1),
    });

    let weak_mmm = Arc::downgrade(&mmm);

    MatchmakeManager::initialize_garbage_collect_thread(weak_mmm).await;

    while let Ok((mut stream, _addr)) = listen.accept().await {
        let buffer = match stream.read_buffer().await{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data buffer: {:?}", e);
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let user_connection_data = match user_connection_data{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data: {:?}", e);
                continue;
            }
        };


        let mmm = mmm.clone();
        task::spawn(async move {
            info!("connection to secure backend established");
            new_rmc_gateway_connection(stream.into(), |r| {
                Arc::new_cyclic(|this| User{
                    this: this.clone(),
                    ip: user_connection_data.prudpsock_addr,
                    pid: user_connection_data.pid,
                    remote: RemoteConsole::new(r),
                    matchmake_manager: mmm,
                    station_url: Default::default()
                })
            });
        });

    }
}