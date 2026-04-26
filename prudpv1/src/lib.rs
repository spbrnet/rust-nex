cfg_if::cfg_if! {
    if #[cfg(feature = "prudpv1")]{
        use proxy_common::ProxyStartupParam;
        pub mod executables;
        pub mod prudp;
        pub async fn start_secure(param: ProxyStartupParam) {
            executables::proxy_secure::start(param).await;
        }

        pub async fn start_insecure(param: ProxyStartupParam) {
            executables::proxy_insecure::start(param).await;
        }
    }
}
