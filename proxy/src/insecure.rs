use proxy_common::ProxyStartupParam;
use rnex_server::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        let param = ProxyStartupParam::new(proxy_common::ProxyType::Insecure)
            .expect("unable to get startup parameters");

        proxy::start_insecure(param).await;
        Ok(())
    })
    .await;
}
