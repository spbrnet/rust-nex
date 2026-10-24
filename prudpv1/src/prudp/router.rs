use crate::prudp::packet::{PRUDPV1Header, PRUDPV1Packet};
use crate::prudp::router::Error::VirtualPortTaken;
use crate::prudp::socket::{AnyInternalSocket, CryptoHandler, ExternalSocket, new_socket_pair};
use rnex_prudp::virtual_port::VirtualPort;
use std::io;
use std::io::Cursor;
use std::marker::PhantomData;
use std::net::SocketAddr::V4;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{error, info, warn};

const MAX_DATAGRAM_SIZE: usize = 65_507;
const MAX_PACKETS_PER_DATAGRAM: usize = 64;
const MIN_PACKET_SIZE: usize = size_of::<PRUDPV1Header>() + 16;

fn has_complete_first_packet(data: &[u8]) -> bool {
    if data.len() < MIN_PACKET_SIZE || data[0..2] != [0xEA, 0xD0] || data[2] != 1 {
        return false;
    }

    let first_packet_size = MIN_PACKET_SIZE
        + usize::from(data[3])
        + usize::from(u16::from_le_bytes([data[4], data[5]]));
    first_packet_size <= data.len()
}

pub struct Router {
    endpoints: RwLock<[Option<Arc<dyn AnyInternalSocket>>; 16]>,
    //running: AtomicBool,
    socket: Arc<UdpSocket>,
    malformed_datagrams: AtomicU64,
    packet_limit_hits: AtomicU64,
    busy_drops: AtomicU64,
    unknown_endpoints: AtomicU64,
    _no_outside_construction: PhantomData<()>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("tried to register socket to a port which is already taken (port: {0})")]
    VirtualPortTaken(u8),
}

impl Router {
    async fn process_prudp_packets(self: Arc<Self>, addr: SocketAddrV4, udp_message: Vec<u8>) {
        let mut stream = Cursor::new(&udp_message);

        for _ in 0..MAX_PACKETS_PER_DATAGRAM {
            if stream.position() as usize == udp_message.len() {
                return;
            }

            let packet = match PRUDPV1Packet::new(&mut stream) {
                Ok(p) => p,
                Err(_) => {
                    self.malformed_datagrams.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            let connection = packet.source_sockaddr(addr);

            let endpoints = self.endpoints.read().await;

            let Some(endpoint) =
                endpoints[packet.header.destination_port.get_port_number() as usize].as_ref()
            else {
                self.unknown_endpoints.fetch_add(1, Ordering::Relaxed);
                continue;
            };

            let endpoint = endpoint.clone();

            // Dont keep the locked structure for too long
            drop(endpoints);

            endpoint.receive_packet(connection, packet).await;
        }

        if stream.position() as usize != udp_message.len() {
            self.packet_limit_hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn server_thread_send_entry(this: Weak<Self>, socket: Arc<UdpSocket>) {
        info!("starting datagram thread");
        let permits = Arc::new(tokio::sync::Semaphore::new(64));
        let mut msg_buffer = vec![0u8; MAX_DATAGRAM_SIZE];

        while let Some(this) = this.upgrade() {
            let (len, addr) = select! {
                r = socket.recv_from(&mut msg_buffer) => {
                    match r { Ok(v) => v, Err(e) => { error!(%e, "UDP receive failed"); continue; } }
                }
                _ = sleep(Duration::from_secs(5)) => {
                    continue;
                }
            };

            let V4(addr) = addr else {
                error!("somehow got ipv6 packet...? ignoring");
                continue;
            };

            let current_msg = &msg_buffer[0..len];

            if !has_complete_first_packet(current_msg) {
                this.malformed_datagrams.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            let Ok(permit) = permits.clone().try_acquire_owned() else {
                this.busy_drops.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let message = current_msg.to_vec();
            tokio::spawn(async move {
                let _permit = permit;
                this.process_prudp_packets(addr, message).await;
            });
        }

        println!("exitting datagram")
    }

    pub async fn new(addr: SocketAddrV4) -> io::Result<(Arc<Self>, JoinHandle<()>)> {
        // trace!("starting router on {}", addr);

        let socket = Arc::new(UdpSocket::bind(addr).await?);

        let own_impl = Router {
            endpoints: Default::default(),
            // running: AtomicBool::new(true),
            socket: socket.clone(),
            malformed_datagrams: AtomicU64::new(0),
            packet_limit_hits: AtomicU64::new(0),
            busy_drops: AtomicU64::new(0),
            unknown_endpoints: AtomicU64::new(0),
            _no_outside_construction: Default::default(),
        };

        let arc = Arc::new(own_impl);

        {
            let router = Arc::downgrade(&arc);
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(60)).await;
                    let Some(router) = router.upgrade() else {
                        break;
                    };

                    let malformed = router.malformed_datagrams.swap(0, Ordering::Relaxed);
                    let packet_limit = router.packet_limit_hits.swap(0, Ordering::Relaxed);
                    let busy = router.busy_drops.swap(0, Ordering::Relaxed);
                    let unknown_endpoint = router.unknown_endpoints.swap(0, Ordering::Relaxed);
                }
            });
        }

        let task = {
            let socket = socket.clone();
            let server = Arc::downgrade(&arc);

            tokio::spawn(async {
                Self::server_thread_send_entry(server, socket).await;
            })
        };

        {
            let _socket = socket.clone();
            let _server = arc.clone();

            tokio::spawn(async {
                //server thread sender entry
                // todo: make this run in the socket cause that makes more sense
                //server.server_thread_recieve_entry(socket).await;
            });
        }

        Ok((arc, task))
    }

    pub fn get_udp_socket(&self) -> Arc<UdpSocket> {
        self.socket.clone()
    }

    // This will remove a socket from the router, this renders all instances of that socket unable
    // to recieve any more data making the error out on trying to for example recieve connections
    pub async fn remove_socket(&self, virtual_port: VirtualPort) {
        self.endpoints.write().await[virtual_port.get_port_number() as usize] = None;
    }

    // returns Some(()) i
    pub async fn add_socket<E: CryptoHandler>(
        &self,
        virtual_port: VirtualPort,
        encryption: E,
    ) -> Result<ExternalSocket<E>, Error> {
        let mut endpoints = self.endpoints.write().await;

        let idx = virtual_port.get_port_number() as usize;

        // dont create the socket if we dont need to
        if !endpoints[idx].is_none() {
            return Err(VirtualPortTaken(idx as u8));
        }

        let (internal, external) = new_socket_pair(virtual_port, encryption, self.socket.clone());

        endpoints[idx] = Some(internal);

        Ok(external)
    }

    pub fn get_own_address(&self) -> SocketAddrV4 {
        match self
            .socket
            .local_addr()
            .expect("unable to get socket address")
        {
            SocketAddr::V4(v4) => v4,
            _ => unreachable!(),
        }
    }
}
