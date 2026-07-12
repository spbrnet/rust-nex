use crate::prudp::packet::PRUDPV1Packet;
use crate::prudp::router::Error::VirtualPortTaken;
use crate::prudp::socket::{AnyInternalSocket, CryptoHandler, ExternalSocket, new_socket_pair};
use log::{error, info};
use std::io;
use std::io::Cursor;
use std::marker::PhantomData;
use std::net::SocketAddr::V4;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::{Arc, Weak};
use std::time::Duration;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;

pub struct Router {
    endpoints: RwLock<[Option<Arc<dyn AnyInternalSocket>>; 16]>,
    //running: AtomicBool,
    socket: Arc<UdpSocket>,
    _no_outside_construction: PhantomData<()>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("tried to register socket to a port which is already taken (port: {0})")]
    VirtualPortTaken(u8),
}

impl Router {
    async fn process_prudp_packets<'a>(
        self: Arc<Self>,
        _socket: Arc<UdpSocket>,
        addr: SocketAddrV4,
        udp_message: Vec<u8>,
    ) {
        let mut stream = Cursor::new(&udp_message);

        while stream.position() as usize != udp_message.len() {
            let packet = match PRUDPV1Packet::new(&mut stream) {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        "Somebody({}) is fucking with the servers or their connection is bad (reason: {})",
                        addr, e
                    );
                    break;
                }
            };

            let connection = packet.source_sockaddr(addr);

            let endpoints = self.endpoints.read().await;

            let Some(endpoint) =
                endpoints[packet.header.destination_port.get_port_number() as usize].as_ref()
            else {
                error!(
                    "connection to invalid endpoint({}) attempted by {}",
                    packet.header.destination_port.get_port_number(),
                    connection.regular_socket_addr
                );
                continue;
            };

            let endpoint = endpoint.clone();

            // Dont keep the locked structure for too long
            drop(endpoints);

            tokio::spawn(async move { endpoint.receive_packet(connection, packet).await });
        }
    }

    async fn server_thread_send_entry(this: Weak<Self>, socket: Arc<UdpSocket>) {
        info!("starting datagram thread");

        while let Some(this) = this.upgrade() {
            // yes we actually allow the max udp to be read lol
            let mut msg_buffer = vec![0u8; 65507];

            let (len, addr) = select! {
                r = socket.recv_from(&mut msg_buffer) => {
                    r.expect("Datagram thread crashed due to unexpected error from recv_from")
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

            tokio::spawn(this.process_prudp_packets(socket.clone(), addr, current_msg.to_vec()));
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
            _no_outside_construction: Default::default(),
        };

        let arc = Arc::new(own_impl);

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
