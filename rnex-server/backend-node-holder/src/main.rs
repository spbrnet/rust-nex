use rnex_node_holder::NodeHolderModule;
use rnex_reggie_protos::reggie::EdgeNodeHolderConnectOption;
use rnex_server::launch_rnex_module_server;

#[tokio::main]
async fn main() {
    launch_rnex_module_server! {
        EdgeNodeHolderConnectOption;
        NodeHolderModule
    }
}
