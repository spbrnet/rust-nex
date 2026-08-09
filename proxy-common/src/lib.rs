use rnex_prudp::{socket_addr::PRUDPSockAddr, virtual_port::VirtualPort};
use rnex_rmc::{
    RemoteDisconnectable, RmcCallable, RmcConnection, RmcPureRemoteObject,
    serialization::RmcSerialize,
};
use rnex_server::{ConnectionInitData, try_get_ip};
use rnex_util::{PID, SendingBufferConnection, SplittableBufferConnection, UnitPacketWrite};
use std::{
    env::{self, VarError},
    fmt::Debug,
    net::{AddrParseError, Ipv4Addr, SocketAddr, SocketAddrV4},
    ops::Deref,
    panic,
    str::FromStr,
    sync::LazyLock,
};
use thiserror::Error;
use tokio::net::TcpStream;
use tracing::{error, info};

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
        "error error getting public ip address: \n\tattempted to read from env var \"SERVER_IP_PUBLIC\" and got: {0}\n\tfor other attempts check logs"
    )]
    PubAddrGetErr(Box<Self>),
}
impl Into<Error> for (&'static str, AddrParseError) {
    fn into(self) -> Error {
        Error::AddrParse(self.0, self.1)
    }
}

pub struct ProxyStartupParam {
    pub forward_destination: SocketAddr,
    // pub edge_node_holder: SocketAddr,
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
                .ok_or(Error::PubAddrGetErr(Box::new(e)))?,
        };

        Ok(Self {
            forward_destination: try_get_env("FORWARD_DESTINATION")?,
            // edge_node_holder: try_get_env("EDGE_NODE_HOLDER")?,
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
impl<T: RemoteDisconnectable + Debug, C: FnOnce() + Send + Sync + 'static> Debug
    for OnRemoteDrop<T, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tuple_builder = f.debug_tuple("OnRemoteDrop");
        tuple_builder.field(&self.0);
        tuple_builder.finish_non_exhaustive()
    }
}
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
    #[allow(dead_code)]
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
        _rest: &[u8],
    ) -> impl Future<Output = bool> + Send {
        // maybe respond with not implemented or something
        async { false }
    }
}

impl<T: RemoteDisconnectable, C: FnOnce() + Send + Sync + 'static> Drop for OnRemoteDrop<T, C> {
    fn drop(&mut self) {
        self.1.take().unwrap()();
    }
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
        addr: addr.regular_socket_addr,
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
