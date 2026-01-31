cfg_if::cfg_if! {
    if #[cfg(feature = "prudpv1")]{
        use proxy_common::{ProxyStartupParam, setup_edge_node_connection};
        pub mod executables;
        pub mod prudp;
        pub async fn start_secure(param: ProxyStartupParam) {
            executables::proxy_secure::start().await;
        }

        pub async fn start_insecure(param: ProxyStartupParam) {
            executables::proxy_insecure::start().await;
        }
    }
}
