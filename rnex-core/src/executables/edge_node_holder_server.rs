use std::io::Cursor;
use std::net::SocketAddrV4;
use std::sync::{Arc, Weak};
use log::error;
use macros::rmc_struct;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use rnex_core::common::setup;
use rnex_core::executables::common::{OWN_IP_PRIVATE, SERVER_PORT};
use rnex_core::reggie::{EdgeNodeHolderConnectOption, EdgeNodeManagement, LocalEdgeNodeHolder};
use rnex_core::rmc::protocols::new_rmc_gateway_connection;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::util::SplittableBufferConnection;
use rnex_core::rmc::structures::RmcSerialize;

#[rmc_struct(EdgeNodeHolder)]
struct EdgeNode{
    data_holder: Arc<DataHolder>,
    address: SocketAddrV4
}

impl EdgeNodeManagement for EdgeNode{
    async fn get_url(&self, seed: u64) -> Result<SocketAddrV4, ErrorCode> {
        self.data_holder.get_url(seed).await
    }
}

#[rmc_struct(EdgeNodeHolder)]
#[derive(Default)]
struct DataHolder{
    edge_nodes: RwLock<Vec<Weak<EdgeNode>>>
}

impl EdgeNodeManagement for DataHolder{
    async fn get_url(&self, seed: u64) -> Result<SocketAddrV4, ErrorCode> {
        let nodes = self.edge_nodes.read().await;

        let nodes: Vec<_> = nodes.iter().filter_map(|n| n.upgrade()).collect();

        // avoid a devide by zero
        if nodes.len() == 0{
            return Err(ErrorCode::Core_InvalidIndex);
        };

        let node = &nodes[seed as usize % nodes.len()];

        Ok(node.address)
    }
}

#[tokio::main]
async fn main() {
    setup();

    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT)).await.unwrap();

    let holder: Arc<DataHolder> = Default::default();

    while let Ok((mut stream, addr)) = listen.accept().await {
        let mut conn: SplittableBufferConnection = stream.into();

        let Some(data) = conn.recv().await else {
            continue;
        };

        let Ok(data) = EdgeNodeHolderConnectOption::deserialize(&mut Cursor::new(data)) else {
            continue;
        };

        let holder = holder.clone();

        match data{
            EdgeNodeHolderConnectOption::DontRegister => {

                new_rmc_gateway_connection(conn, |_| holder);
            },
            EdgeNodeHolderConnectOption::Register(address) => {
                let edge_node = EdgeNode{
                    address,
                    data_holder: holder.clone()
                };

                let node = new_rmc_gateway_connection(conn, move |_| Arc::new(edge_node));

                let mut nodes = holder.edge_nodes.write().await;
                nodes.push(Arc::downgrade(&node));
            }
        }


    }
}