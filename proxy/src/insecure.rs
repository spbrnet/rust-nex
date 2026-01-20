use rnex_core::common::setup;

#[tokio::main]
async fn main() {
    setup();

    proxy::start_insecure(ProxyStartupParam::new()).await;
}
