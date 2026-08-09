use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "prudpv0")] {
        use tracing::info;
        use proxy_common::ProxyStartupParam;
        use std::env;
        use std::net::SocketAddrV4;
        use std::sync::{Arc, LazyLock};

        use crate::crypto::{Crypto, Insecure, Secure};
        use crate::server::Server;

        mod crypto;
        mod packet;
        mod server;

        // pub static EDGE_NODE_HOLDER: LazyLock<SocketAddrV4> = LazyLock::new(|| {
        //     env::var("EDGE_NODE_HOLDER")
        //         .ok()
        //         .and_then(|s| s.parse().ok())
        //         .expect("EDGE_NODE_HOLDER not set")
        // });

        pub static FORWARD_DESTINATION: LazyLock<SocketAddrV4> = LazyLock::new(|| {
            env::var("FORWARD_DESTINATION")
                .ok()
                .and_then(|s| s.parse().ok())
                .expect("FORWARD_DESTINATION not set")
        });
        //same as with prudpv1 this is responsible for handeling the different cryptography
        //implementations, e.g. secure and insecure(this also includes special cases like friends)

        async fn start_proxy<T: Crypto>(param: ProxyStartupParam) {
            info!("binding to socket");

            let server: Arc<Server<T>> = Arc::new(Server::new(param).await);

            info!("waiting on packets");
            server.run_task().await;
        }

        pub async fn start_secure(param: ProxyStartupParam) {
            start_proxy::<Secure>(param).await;
        }

        pub async fn start_insecure(param: ProxyStartupParam) {
            start_proxy::<Insecure>(param).await;
        }
    }
}
