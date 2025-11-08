use rnex_core::reggie::RemoteEdgeNodeHolder;
use once_cell::sync::Lazy;
use rnex_core::common::setup;
use rnex_core::rmc::structures::RmcSerialize;
use std::env;
use std::net::SocketAddrV4;
use std::sync::Arc;
use tokio::net::TcpStream;
use rnex_core::executables::common::{SECURE_SERVER_ACCOUNT, new_simple_backend};
use rnex_core::nex::auth_handler::AuthHandler;
use rnex_core::reggie::EdgeNodeHolderConnectOption::DontRegister;
use rnex_core::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rnex_core::util::SplittableBufferConnection;


pub static FORWARD_EDGE_NODE_HOLDER: Lazy<SocketAddrV4> = Lazy::new(||{
    env::var("FORWARD_EDGE_NODE_HOLDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("FORWARD_EDGE_NODE_HOLDER not set")
});



#[tokio::main]
async fn main() {
    setup();

    let conn = TcpStream::connect(&*FORWARD_EDGE_NODE_HOLDER).await.unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(DontRegister.to_data()).await;

    let conn = new_rmc_gateway_connection(conn, |r| Arc::new(OnlyRemote::<RemoteEdgeNodeHolder>::new(r)));

    new_simple_backend(move |_, _|{
        let controller = conn.clone();
        Arc::new(AuthHandler {
            destination_server_acct: &SECURE_SERVER_ACCOUNT,
            build_name: "branch:origin/project/wup-agmj build:3_8_15_2004_0",
            control_server: controller
        })
    }).await;
}
