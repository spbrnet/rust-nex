use bytemuck::{Pod, Zeroable};
use log::{error, info, warn};
use proxy_common::{ProxyStartupParam, setup_edge_node_connection};
use rnex_core::executables::common::{OWN_IP_PRIVATE, OWN_IP_PUBLIC, SERVER_PORT};
use rnex_core::prudp::types_flags::TypesFlags;
use rnex_core::prudp::types_flags::types::SYN;
use rnex_core::prudp::virtual_port::VirtualPort;
use rnex_core::reggie::EdgeNodeHolderConnectOption::Register;
use rnex_core::reggie::RemoteEdgeNodeHolder;
use rnex_core::rmc::protocols::{OnlyRemote, new_rmc_gateway_connection};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::util::SplittableBufferConnection;
use std::env;
use std::net::SocketAddrV4;
use std::process::abort;
use std::sync::{Arc, LazyLock};
use tokio::net::UdpSocket;

use crate::crypto::{Crypto, Insecure, Secure};
use crate::packet::PRUDPV0Packet;
use crate::server::Server;

mod crypto;
mod packet;
mod server;

pub static EDGE_NODE_HOLDER: LazyLock<SocketAddrV4> = LazyLock::new(|| {
    env::var("EDGE_NODE_HOLDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("EDGE_NODE_HOLDER not set")
});

pub static FORWARD_DESTINATION: LazyLock<SocketAddrV4> = LazyLock::new(|| {
    env::var("FORWARD_DESTINATION")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("FORWARD_DESTINATION not set")
});
//same as with prudpv1 this is responsible for handeling the different cryptography
//implementations, e.g. secure and insecure(this also includes special cases like friends)

async fn start_proxy<T: Crypto>(param: ProxyStartupParam) {
    setup_edge_node_connection(&param, || abort());

    info!("creating cryptography instance");
    let mut crypto = Arc::new(T::new());
    info!("binding to socket");

    let server: Arc<Server<T>> = Arc::new(Server::new().await);

    info!("waiting on packets");
    server.run_task().await;
}

pub async fn start_secure(param: ProxyStartupParam) {
    start_proxy::<Secure>(param).await;
}

pub async fn start_insecure(param: ProxyStartupParam) {
    start_proxy::<Insecure>(param).await;
}
