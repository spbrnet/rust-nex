use proxy_common::ProxyStartupParam;
use rnex_server::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        let param = ProxyStartupParam::new(proxy_common::ProxyType::Secure)
            .expect("unable to get startup parameters");

        // setup_edge_node_connection(&param, edge_node_dc_callback).await;
        proxy::start_secure(param).await;
        Ok(())
    })
    .await;
}
