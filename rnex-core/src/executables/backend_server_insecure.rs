use rnex_core::reggie::{RemoteEdgeNodeHolder, UnitPacketRead};
use log::{error, info};
use once_cell::sync::Lazy;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::pki_types::{CertificateDer, TrustAnchor, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{ClientCertVerifierBuilder, WebPkiClientVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, Error, RootCertStore, ServerConfig, ServerConnection,
    SignatureScheme,
};
use rustls_pki_types::PrivateKeyDer;
use rnex_core::common::setup;
use std::borrow::ToOwned;
use std::{env, fs};
use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use macros::{method_id, rmc_proto, rmc_struct};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task;
use tokio_rustls::TlsAcceptor;
use rnex_core::define_rmc_proto;
use rnex_core::executables::common::{OWN_IP_PRIVATE, SECURE_SERVER_ACCOUNT, SERVER_PORT};
use rnex_core::nex::auth_handler::AuthHandler;
use rnex_core::reggie::EdgeNodeHolderConnectOption::DontRegister;
use rnex_core::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use rnex_core::util::SplittableBufferConnection;


pub static FORWARD_EDGE_NODE_HOLDER: Lazy<SocketAddrV4> = Lazy::new(||{
    env::var("FORWARD_EDGE_NODE_HOLDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SECURE_EDGE_NODE_HOLDER not set")
});



#[tokio::main]
async fn main() {
    setup();

    let conn = TcpStream::connect(&*FORWARD_EDGE_NODE_HOLDER).await.unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(DontRegister.to_data()).await;

    let conn = new_rmc_gateway_connection(conn, |r| Arc::new(OnlyRemote::<RemoteEdgeNodeHolder>::new(r)));

    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT)).await.unwrap();



    while let Ok((mut stream, addr)) = listen.accept().await {
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
        let controller = conn.clone();
        task::spawn(async move {
            info!("connection to secure backend established");
            new_rmc_gateway_connection(stream.into(), |_| {
                Arc::new(AuthHandler {
                    destination_server_acct: &SECURE_SERVER_ACCOUNT,
                    build_name: "branch:origin/project/wup-agmj build:3_8_15_2004_0",
                    control_server: controller
                })
            });
        });

    }
}
