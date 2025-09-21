use std::env;
use std::net::SocketAddrV4;
use once_cell::sync::Lazy;

pub static EDGE_NODE_HOLDER: Lazy<SocketAddrV4> = Lazy::new(||{
    env::var("EDGE_NODE_HOLDER")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("EDGE_NODE_HOLDER not set")
});

pub static FORWARD_DESTINATION: Lazy<SocketAddrV4> =
    Lazy::new(||
        env::var("FORWARD_DESTINATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .expect("FORWARD_DESTINATION not set")
    );
