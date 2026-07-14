use std::{
    convert::Infallible,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Weak},
};

use rnex_reggie_protos::reggie::{
    EdgeNodeHolderConnectOption, EdgeNodeManagement, LocalEdgeNodeHolder,
};
use rnex_rmc::{response::ErrorCode, rmc_struct};
use rnex_server::{
    PassthroughInitModule, RnexManager, RnexModule, WeakPassthroughInitModule,
};
use tokio::sync::RwLock;

#[derive(Debug)]
#[rmc_struct(EdgeNodeHolder)]
pub struct EdgeNode {
    em: PassthroughInitModule<NodeHolderManager>,
    address: SocketAddrV4,
}

impl EdgeNodeManagement for EdgeNode {
    async fn get_url(&self, seed: u64) -> Result<SocketAddrV4, ErrorCode> {
        let nodes = self.em.edge_nodes.read().await;

        let nodes: Vec<_> = nodes.iter().filter_map(|n| n.upgrade()).collect();

        // avoid a devide by zero
        if nodes.len() == 0 {
            return Err(ErrorCode::Core_InvalidIndex);
        };

        let node = &nodes[seed as usize % nodes.len()];

        Ok(node.address)
    }
}

#[derive(Default, Debug)]
pub struct NodeHolderManager {
    edge_nodes: RwLock<Vec<Weak<EdgeNode>>>,
}
#[derive(Default, Debug)]
pub struct NodeHolderModule;

impl RnexManager for NodeHolderManager {
    type User = Arc<EdgeNode>;
    type InitData = EdgeNodeHolderConnectOption;
    async fn init_new_user(
        this: PassthroughInitModule<Self>,
        _mod_holder: &rnex_server::ModuleHolder,
        _remote: &rnex_rmc::RmcConnection,
        init_data: &Self::InitData,
        _: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        match init_data {
            EdgeNodeHolderConnectOption::DontRegister => Arc::new(EdgeNode {
                em: this,
                address: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            }),
            EdgeNodeHolderConnectOption::Register(socket_addr_v4) => {
                let node = Arc::new(EdgeNode {
                    em: this.clone(),
                    address: *socket_addr_v4,
                });
                this.edge_nodes.write().await.push(Arc::downgrade(&node));

                node
            }
        }
    }
}

impl RnexModule for NodeHolderModule {
    type Manager = NodeHolderManager;
    type InitError = Infallible;

    async fn create_manager(
        _: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(NodeHolderManager::default())
    }
}
