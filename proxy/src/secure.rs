use proxy::edge_node_dc_callback;
use proxy_common::{ProxyStartupParam, setup_edge_node_connection};
use rnex_core::common::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        let param = ProxyStartupParam::new(proxy_common::ProxyType::Secure)
            .expect("unable to get startup parameters");

        setup_edge_node_connection(&param, edge_node_dc_callback).await;
        proxy::start_secure(param).await;
    })
    .await;
}
