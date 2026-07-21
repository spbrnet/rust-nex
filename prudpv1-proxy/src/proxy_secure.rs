use proxy_common::{ProxyStartupParam, RNEX_ACCESS_KEY};
use prudpv1::prudp::{router::Router, secure::Secure};
use rnex_prudp::virtual_port::VirtualPort;
use rnex_server::ConnectionInitData;
use rnex_server::rmc::serialization::RmcSerialize;
use rnex_util::account::Account;
use rnex_util::{UnitPacketRead, UnitPacketWrite};
use std::ops::Deref;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task;
use tokio::time::sleep;
use tracing::error;
use tokio::io::AsyncWriteExt;

pub async fn start(param: ProxyStartupParam) {
    let (router_secure, _) = Router::new(param.self_private)
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(
            VirtualPort::new(1, 10),
            Secure(
                RNEX_ACCESS_KEY,
                Account::from_nexact(2, "Quazal Rendez-Vous")
                    .await
                    .expect("failed to get account"),
            ),
        )
        .await
        .expect("unable to add socket");

    loop {
        let Some(mut conn) = socket_secure.accept().await else {
            error!("server crashed");
            return;
        };

        task::spawn(async move {
            // todo: add support for checking this to nex-account
            /*
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
            } */

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
                        addr: conn.socket_addr.regular_socket_addr,
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

                        if data == [0,0,0,0,0] {
                            continue;
                        }

                        if conn.send(data).await == None{
                            break 'a;
                        }
                    },
                    _ = sleep(Duration::from_secs(10)) => {
                        stream.write_all(&[0,0,0,0,0].to_vec()).await.ok();
                    }
                }
            }
            conn.deref().close_connection().await;
        });
    }
}
