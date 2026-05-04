use log::{error, info};
use rnex_core::{
    PID,
    executables::common::try_get_ip,
    prudp::{socket_addr::PRUDPSockAddr, virtual_port::VirtualPort},
    reggie::{RemoteEdgeNodeHolder, UnitPacketWrite},
    rmc::{
        protocols::{
            RemoteDisconnectable, RmcCallable, RmcConnection, RmcPureRemoteObject,
            new_rmc_gateway_connection,
        },
        structures::RmcSerialize,
    },
    rnex_proxy_common::ConnectionInitData,
    util::{SendingBufferConnection, SplittableBufferConnection},
};
use std::{
    env::{self, VarError},
    error,
    net::{AddrParseError, Ipv4Addr, SocketAddr, SocketAddrV4},
    ops::Deref,
    panic,
    str::FromStr,
    sync::{Arc, LazyLock},
};
use thiserror::Error;
use tokio::net::TcpStream;

const RNEX_DEFAULT_PORT: u16 = match u16::from_str_radix(env!("RNEX_DEFAULT_PORT"), 10) {
    Ok(v) => v,
    Err(_) => panic!("unable to get default port from env"),
};

pub const RNEX_ACCESS_KEY: &'static str = env!("RNEX_ACCESS_KEY");

#[derive(Error, Debug)]
pub enum Error {
    #[error("error getting environment variable \"{0}\": {1}")]
    UnableToGetEnv(&'static str, VarError),
    #[error("error parsing ip address environment variable \"{0}\": {1}")]
    AddrParse(&'static str, AddrParseError),
    #[error(
        "error error getting public ip address: \n\tattempted to read from env var \"SERVER_IP_PUBLIC\" and got: {0} \n\tattempted to request from internet and failed with: {1}"
    )]
    PubAddrGetErr(Box<Self>, Box<dyn error::Error>),
}
impl Into<Error> for (&'static str, AddrParseError) {
    fn into(self) -> Error {
        Error::AddrParse(self.0, self.1)
    }
}

pub struct ProxyStartupParam {
    pub forward_destination: SocketAddr,
    pub edge_node_holder: SocketAddr,
    pub self_public: SocketAddrV4,
    pub self_private: SocketAddrV4,
    pub virtual_port: VirtualPort,
}

fn try_get_env<T: FromStr>(name: &'static str) -> Result<T, Error>
where
    (&'static str, T::Err): Into<Error>,
{
    T::from_str(&env::var(name).map_err(|e| Error::UnableToGetEnv(name, e))?)
        .map_err(|e| (name, e).into())
}

pub enum ProxyType {
    Insecure,
    Secure,
}
const VIRTUAL_PORT_INSECURE: LazyLock<VirtualPort> =
    LazyLock::new(|| VirtualPort::parse(env!("RNEX_VIRTUAL_PORT_INSECURE")).unwrap());
const VIRTUAL_PORT_SECURE: LazyLock<VirtualPort> =
    LazyLock::new(|| VirtualPort::parse(env!("RNEX_VIRTUAL_PORT_SECURE")).unwrap());
impl ProxyStartupParam {
    #[inline(always)]
    pub fn new(prox_ty: ProxyType) -> Result<Self, Error> {
        let port = RNEX_DEFAULT_PORT
            + match prox_ty {
                ProxyType::Insecure => 0,
                ProxyType::Secure => 1,
            };
        let self_private = try_get_env("SERVER_IP_PRIVATE")
            .unwrap_or(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
        let self_public: SocketAddrV4 = match try_get_env("SERVER_IP_PUBLIC") {
            Ok(v) => v,
            Err(e) => try_get_ip()
                .map(|v| SocketAddrV4::new(v, self_private.port()))
                .map_err(move |v| Error::PubAddrGetErr(Box::new(e), v))?,
        };

        Ok(Self {
            forward_destination: try_get_env("FORWARD_DESTINATION")?,
            edge_node_holder: try_get_env("EDGE_NODE_HOLDER")?,
            self_private,
            self_public,
            virtual_port: match prox_ty {
                ProxyType::Insecure => *VIRTUAL_PORT_INSECURE,
                ProxyType::Secure => *VIRTUAL_PORT_SECURE,
            },
        })
    }
}

struct OnRemoteDrop<T: RemoteDisconnectable, C: FnOnce() + Send + Sync + 'static>(T, Option<C>);
impl<T: RemoteDisconnectable, C: FnOnce() + Send + Sync + 'static> Deref for OnRemoteDrop<T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// if we had something like a thread safe OnceConsume (basically the opposite of OnceLock)
// we could make C be an FnOnce
impl<T: RemoteDisconnectable + RmcPureRemoteObject, C: FnOnce() + Send + Sync + 'static>
    OnRemoteDrop<T, C>
{
    pub fn new(conn: RmcConnection, drop_func: C) -> Self {
        Self(T::new(conn), Some(drop_func))
    }

    #[allow(dead_code)]
    pub async fn disconnect(&self) {
        self.0.disconnect().await;
    }
}

impl<T: RemoteDisconnectable, C: FnOnce() + Send + Sync + 'static> RmcCallable
    for OnRemoteDrop<T, C>
{
    fn rmc_call(
        &self,
        _responder: &SendingBufferConnection,
        _protocol_id: u16,
        _method_id: u32,
        _call_id: u32,
        _rest: Vec<u8>,
    ) -> impl Future<Output = ()> + Send {
        // maybe respond with not implemented or something
        async {}
    }
}

impl<T: RemoteDisconnectable, C: FnOnce() + Send + Sync + 'static> Drop for OnRemoteDrop<T, C> {
    fn drop(&mut self) {
        self.1.take().unwrap()();
    }
}

pub async fn setup_edge_node_connection(
    param: &ProxyStartupParam,
    shutdown_callback: impl FnOnce() + Send + Sync + 'static,
) {
    let conn = tokio::net::TcpStream::connect(&param.edge_node_holder)
        .await
        .unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(
        rnex_core::reggie::EdgeNodeHolderConnectOption::Register(param.self_public)
            .to_data()
            .unwrap(),
    )
    .await;

    println!("{:?}", param.self_public);
    //leave the inner object floating so that it gets destroyed once we disconnect
    new_rmc_gateway_connection(conn, move |r| {
        Arc::new(OnRemoteDrop::<RemoteEdgeNodeHolder, _>::new(
            r,
            shutdown_callback,
        ))
    });
}

pub async fn new_backend_connection(
    param: &ProxyStartupParam,
    addr: PRUDPSockAddr,
    pid: PID,
) -> Option<SplittableBufferConnection> {
    info!("attempting to connect to: {}", param.forward_destination);
    let mut stream = match TcpStream::connect(param.forward_destination).await {
        Ok(v) => v,
        Err(e) => {
            error!("unable to establish connection to backend: {}", e);
            return None;
        }
    };

    let data = ConnectionInitData {
        prudpsock_addr: addr,
        pid: pid,
    }
    .to_data()
    .unwrap();

    if let Err(e) = stream.send_buffer(&data).await {
        error!("unable to send establishment data to backend: {}", e);
        return None;
    };

    Some(stream.into())
}

#[cfg(test)]
mod test {
    use crate::{VIRTUAL_PORT_INSECURE, VIRTUAL_PORT_SECURE};

    #[test]
    fn test_virtual_port_correct() {
        println!("{:?}", VIRTUAL_PORT_INSECURE);
        println!("{:?}", VIRTUAL_PORT_SECURE);
    }
}
