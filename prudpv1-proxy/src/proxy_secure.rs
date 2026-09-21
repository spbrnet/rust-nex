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

pub async fn start(param: ProxyStartupParam) {
    let (router_secure, _) = Router::new(param.self_private)
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(
            VirtualPort::new(1, 10),
            Secure(
                RNEX_ACCESS_KEY,
                Account::from_password_env(2, "Quazal Rendez-Vous", "RNEX_SERVER_PASSWORD")
                    .expect("RNEX_SERVER_PASSWORD is required"),
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
            let stream = match TcpStream::connect(param.forward_destination).await {
                Ok(v) => v,
                Err(e) => {
                    error!("unable to connect: {}", e);
                    return;
                }
            };

            let (mut read_half, mut write_half) = stream.into_split();

            if let Err(e) = write_half
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

            let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);

            let reader_handle = task::spawn(async move {
                loop {
                    match read_half.read_buffer().await {
                        Ok(data) => {
                            if data == [0, 0, 0, 0, 0] {
                                continue;
                            }
                            if tx.send(data).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("error receiving data from backend: {}", e);
                            break;
                        }
                    }
                }
            });

            let keepalive_data = vec![0u8; 5];
            'a: loop {
                tokio::select! {
                    data = conn.recv() => {
                        let Some(data) = data else {
                            break 'a;
                        };

                        if let Err(e) = write_half.send_buffer(&data[..]).await {
                            error!("error sending data to backend: {}", e);
                            break 'a;
                        }
                    },
                    data = rx.recv() => {
                        let Some(data) = data else {
                            break 'a;
                        };

                        if conn.send(data).await.is_none() {
                            break 'a;
                        }
                    },
                    _ = sleep(Duration::from_secs(10)) => {
                        if let Err(e) = write_half.send_buffer(&keepalive_data).await {
                            error!("failed to send keepalive: {}", e);
                            break 'a;
                        }
                    }
                }
            }

            reader_handle.abort();
            conn.deref().close_connection().await;
        });
    }
}
