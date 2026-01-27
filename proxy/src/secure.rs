use proxy_common::ProxyStartupParam;
use rnex_core::common::setup;

#[tokio::main]
async fn main() {
    setup();
    proxy::start_secure(
        ProxyStartupParam::new(proxy_common::ProxyType::Secure)
            .expect("unable to get startup parameters"),
    )
    .await;
}
