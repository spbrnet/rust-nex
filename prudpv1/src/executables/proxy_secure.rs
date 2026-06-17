use crate::prudp::router::Router;
use crate::prudp::secure::Secure;
use log::error;
use log::warn;
use proxy_common::{ProxyStartupParam, RNEX_ACCESS_KEY};
use rnex_core::executables::common::SECURE_SERVER_ACCOUNT;
use rnex_core::prudp::virtual_port::VirtualPort;
use rnex_core::reggie::UnitPacketRead;
use rnex_core::reggie::UnitPacketWrite;
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use std::ops::Deref;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task;
use tokio::time::sleep;

pub async fn start(param: ProxyStartupParam) {
    let (router_secure, _) = Router::new(param.self_private)
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(
            VirtualPort::new(1, 10),
            Secure(RNEX_ACCESS_KEY, SECURE_SERVER_ACCOUNT.clone()),
        )
        .await
        .expect("unable to add socket");

    loop {
        let Some(mut conn) = socket_secure.accept().await else {
            error!("server crashed");
            return;
        };

        task::spawn(async move {
            let Ok(mut c) = rnex_core::grpc::account::Client::new().await else {
                error!("failed to initialize gql client");
                return;
            };

            let v = match c.get_user_level(conn.user_id).await {
                Ok(v) => v,
                Err(e) => {
                    error!("failed to get user level: {}", e);
                    return;
                }
            };

            if v < 0 {
                warn!("person with too low account level joined");
                return;
            }

            let mut stream = match TcpStream::connect(param.forward_destination).await {
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

            'a: loop {
                tokio::select! {
                    data = conn.recv() => {
                        let Some(data) = data else {
                            break 'a;
                        };

                        if let Err(e) = stream.send_buffer(&data[..]).await{
                            error!("error sending data to backend: {}", e);
                            break 'a;
                        }
                    },
                    data = stream.read_buffer() => {
                        let data = match data{
                            Ok(d) => d,
                            Err(e) => {
                                error!("error reveiving data from backend: {}", e);
                                break 'a;
                            }
                        };

                        if conn.send(data).await == None{
                            break 'a;
                        }
                    },
                    _ = sleep(Duration::from_secs(10)) => {
                        conn.send([0,0,0,0,0].to_vec()).await;
                    }
                }
            }
            conn.deref().close_connection().await;
        });
    }
}
