#![cfg(feature = "prudpv1")]
use proxy_common::ProxyStartupParam;

pub mod proxy_insecure;
pub mod proxy_secure;

pub async fn start_secure(param: ProxyStartupParam) {
    proxy_secure::start(param).await;
}

pub async fn start_insecure(param: ProxyStartupParam) {
    proxy_insecure::start(param).await;
}
