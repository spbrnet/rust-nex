use proxy::edge_node_dc_callback;
use proxy_common::{ProxyStartupParam, setup_edge_node_connection};
use rnex_server::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        let param = ProxyStartupParam::new(proxy_common::ProxyType::Insecure)
            .expect("unable to get startup parameters");

        // setup_edge_node_connection(&param, edge_node_dc_callback).await;

        proxy::start_insecure(param).await;
        Ok(())
    })
    .await;
}
