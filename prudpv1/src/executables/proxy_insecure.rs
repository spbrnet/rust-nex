use crate::executables::common::{EDGE_NODE_HOLDER, FORWARD_DESTINATION};
use crate::prudp::router::Router;
use crate::prudp::unsecure::Unsecure;
use log::error;
use rnex_core::common::setup;
use rnex_core::executables::common::{OWN_IP_PRIVATE, OWN_IP_PUBLIC, SERVER_PORT};
use rnex_core::prudp::virtual_port::VirtualPort;
use rnex_core::reggie::EdgeNodeHolderConnectOption::Register;
use rnex_core::reggie::RemoteEdgeNodeHolder;
use rnex_core::reggie::UnitPacketRead;
use rnex_core::reggie::UnitPacketWrite;
use rnex_core::rmc::protocols::{OnlyRemote, new_rmc_gateway_connection};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use rnex_core::util::SplittableBufferConnection;
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task;
use tokio::time::sleep;

pub async fn start() {
    /*let conn = tokio::net::TcpStream::connect(&*EDGE_NODE_HOLDER)
        .await
        .unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(
        Register(SocketAddrV4::new(*OWN_IP_PUBLIC, *SERVER_PORT))
            .to_data()
            .unwrap(),
    )
    .await;

    let conn = new_rmc_gateway_connection(conn, |r| {
        Arc::new(OnlyRemote::<RemoteEdgeNodeHolder>::new(r))
    });*/

    let (router_secure, _) = Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(VirtualPort::new(1, 10), Unsecure("6f599f81"))
        .await
        .expect("unable to add socket");

    // let conn = socket_secure.connect(auth_sockaddr).await.unwrap();

    loop {
        let Some(mut conn) = socket_secure.accept().await else {
            error!("server crashed");
            return;
        };

        task::spawn(async move {
            let mut stream = match TcpStream::connect(*FORWARD_DESTINATION).await {
                Ok(v) => v,
                Err(e) => {
                    error!("unable to connect: {}", e);
                    return;
                }
            };

            if let Err(e) = stream
                .send_buffer(
                    &ConnectionInitData {
                        prudpsock_addr: conn.socket_addr,
                        pid: conn.user_id,
                    }
                    .to_data()
                    .unwrap(),
                )
                .await
            {
                error!("error connecting to backend: {}", e);
                return;
            };

            loop {
                tokio::select! {
                    data = conn.recv() => {
                        let Some(data) = data else {
                            return;
                        };

                        if let Err(e) = stream.send_buffer(&data[..]).await{
                            error!("error sending data to backend: {}", e);
                            return;
                        }
                    },
                    data = stream.read_buffer() => {
                        let data = match data{
                            Ok(d) => d,
                            Err(e) => {
                                error!("error reveiving data from backend: {}", e);
                                return;
                            }
                        };

                        if conn.send(data).await == None{
                            return;
                        }
                    },
                    _ = sleep(Duration::from_secs(10)) => {
                        conn.send([0,0,0,0,0].to_vec()).await;
                    }
                }
            }
        });
    }
}
