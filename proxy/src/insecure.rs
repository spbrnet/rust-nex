use proxy_common::ProxyStartupParam;
use rnex_core::common::setup;

#[tokio::main]
async fn main() {
    setup();

    proxy::start_insecure(
        ProxyStartupParam::new(proxy_common::ProxyType::Insecure)
            .expect("unable to get startup parameters"),
    )
    .await;
}
