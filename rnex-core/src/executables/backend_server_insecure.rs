use cfg_if::cfg_if;
use once_cell::sync::Lazy;
use rnex_core::common::setup;
use rnex_core::executables::common::{SECURE_SERVER_ACCOUNT, new_simple_backend};
use rnex_core::executables::regular_backend;
use rnex_core::nex::auth_handler::AuthHandler;
use rnex_core::reggie::EdgeNodeHolderConnectOption::DontRegister;
use rnex_core::reggie::RemoteEdgeNodeHolder;
use rnex_core::rmc::protocols::{OnlyRemote, new_rmc_gateway_connection};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::util::SplittableBufferConnection;
use std::env;
use std::net::SocketAddrV4;
use std::sync::Arc;
use tokio::net::TcpStream;

pub static FORWARD_EDGE_NODE_HOLDER: Lazy<SocketAddrV4> = Lazy::new(|| {
    env::var("FORWARD_EDGE_NODE_HOLDER")
        .ok()
        .and_then(|s| Some(s.parse().unwrap()))
        .expect("FORWARD_EDGE_NODE_HOLDER not set")
});

#[tokio::main]
async fn main() {
    setup();

    cfg_if! {
        if #[cfg(features = "friends")]{

        } else {
            regular_backend::start_regular_backend().await
        }
    }
}
