use std::{
    collections::HashMap,
    hash::Hash,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicU32},
    },
};

use log::{error, info, warn};
use proxy_common::{ProxyStartupParam, new_backend_connection};
use rnex_core::{
    executables::common::{OWN_IP_PRIVATE, SERVER_PORT},
    prudp::{
        socket_addr::PRUDPSockAddr,
        types_flags::{
            TypesFlags,
            flags::{ACK, HAS_SIZE, NEED_ACK},
            types::{CONNECT, DATA, SYN},
        },
        virtual_port::VirtualPort,
    },
    rnex_proxy_common::ConnectionInitData,
    util::{SendingBufferConnection, SplittableBufferConnection},
};
use tokio::{
    net::{TcpSocket, UdpSocket},
    spawn,
    sync::{Mutex, RwLock},
    time::Instant,
};

use crate::{
    crypto::{Crypto, CryptoInstance},
    packet::{PRUDPV0Header, PRUDPV0Packet, new_syn_packet, precalc_size},
};

pub struct InternalConnection<C: CryptoInstance> {
    last_action: Instant,
    crypto_instance: C,
    server_packet_counter: u16,
    client_packet_counter: u16,
    unacknowledged_packets: HashMap<u16, Arc<Vec<u8>>>,
}
pub struct Connection<C: CryptoInstance> {
    alive: AtomicBool,
    session_id: u8,
    target: SendingBufferConnection,
    self_signat: [u8; 4],
    remote_signat: [u8; 4],
    addr: PRUDPSockAddr,
    inner: Mutex<InternalConnection<C>>,
}

impl<C: CryptoInstance> InternalConnection<C> {
    fn next_server_count(&mut self) -> u16 {
        let prev_val = self.server_packet_counter;
        let (val, _) = self.server_packet_counter.overflowing_add(1);
        self.server_packet_counter = val;

        prev_val
    }
}

pub struct Server<C: Crypto> {
    param: ProxyStartupParam,
    socket: UdpSocket,
    crypto: C,
    connections: RwLock<HashMap<PRUDPSockAddr, Arc<Connection<C::Instance>>>>,
}

impl<C: Crypto> Server<C> {
    async fn send_data_packet(&self, conn: &Connection<C::Instance>, data: &[u8]) {
        let type_flags = TypesFlags::default().types(DATA).flags(HAS_SIZE | NEED_ACK);
        let vec = vec![0; precalc_size(type_flags, data.len())];
        let mut packet = PRUDPV0Packet::new(vec);

        let payload = packet.payload_mut().expect("packet malformed in creation");
        payload.copy_from_slice(data);

        let mut inner = conn.inner.lock().await;
        inner.crypto_instance.encrypt_outgoing(payload);
        let packet_signat = inner.crypto_instance.generate_signature(payload);
        let seq = inner.next_server_count();

        *packet.header_mut().expect("packet malformed in creation") = PRUDPV0Header {
            source: self.param.virtual_port,
            destination: conn.addr.virtual_port,
            type_flags,
            session_id: conn.session_id,
            packet_signature: packet_signat,
            sequence_id: seq,
        };
        /* we leave the sequence id as is for now as it defaults to 0 */

        *packet.checksum_mut().expect("packet malformed in creation") =
            self.crypto.calculate_checksum(
                packet
                    .checksummed_data()
                    .expect("packet malformed in creation"),
            );

        let packet_raw = packet.0;

        let packet = Arc::new(packet_raw);

        let packet_ref = Arc::downgrade(&packet);

        inner.unacknowledged_packets.insert(seq, packet);

        drop(inner);

        spawn(async move {
            for n in 0..5 {
                let Some(data) = packet_ref.upgrade() else {
                    return;
                };
                info!("send attempt {}", n);
            }
        });
    }
    async fn connection_thread(
        self: Arc<Self>,
        conn: Arc<Connection<C::Instance>>,
        mut recv: SplittableBufferConnection,
    ) {
        while let Some(data) = recv.recv().await {}
    }
    async fn timeout_thread(self: Arc<Self>, conn: Arc<Connection<C::Instance>>) {
        loop {
            conn
        }
    }
    async fn handle_syn(self: Arc<Self>, packet: PRUDPV0Packet<&[u8]>, addr: PRUDPSockAddr) {
        info!("got syn");
        let header = packet.header().unwrap();

        let signat = addr.calculate_connection_signature();
        let signat = [signat[0], signat[1], signat[2], signat[3]];

        let packet = new_syn_packet(ACK, header.destination, header.source, signat, &self.crypto);
        self.socket.send_to(&packet, addr.regular_socket_addr).await;
    }
    async fn handle_connect(self: Arc<Self>, packet: PRUDPV0Packet<&[u8]>, addr: PRUDPSockAddr) {
        let conn = self.connections.write().await;
        let Some(data) = packet.payload() else {
            warn!("malformed packet from: {:?}", addr.regular_socket_addr);
            return;
        };
        let Some(self_signat) = packet.connection_signature().copied() else {
            warn!(
                "malformed packet(unable to find connection signature) from: {:?}",
                addr
            );
            return;
        };

        let ci = self.crypto.instantiate(data);

        let pid = ci.get_user_id();
        let conn = new_backend_connection(&self.param, addr, pid).await;
        let Some(conn) = conn else {
            error!("unable to connect to backend");
            return;
        };

        let remote_signat = addr.calculate_connection_signature();
        let remote_signat = [
            remote_signat[0],
            remote_signat[1],
            remote_signat[2],
            remote_signat[3],
        ];

        let header = packet.header().expect("header should be validated by now");

        let conn = Arc::new(Connection {
            target: conn.duplicate_sender(),
            remote_signat,
            self_signat,
            addr,
            session_id: header.session_id,
            alive: AtomicBool::new(true),
            inner: Mutex::new(InternalConnection {
                last_action: Instant::now(),
                crypto_instance: ci,
                client_packet_counter: 2,
                server_packet_counter: 1,
                unacknowledged_packets: HashMap::new(),
            }),
        });
    }
    async fn process_packet<'a>(self: Arc<Self>, packet: PRUDPV0Packet<&[u8]>, addr: SocketAddrV4) {
        if !packet.check_checksum(&self.crypto) {
            warn!("invalid checksum from: {}", addr);
            return;
        }

        let Some(header) = packet.header() else {
            warn!("malformatted packet from: {}", addr);
            return;
        };

        let addr = PRUDPSockAddr::new(addr, header.source);
        println!("{:?}", header);
        match header.type_flags.get_types() {
            SYN => {
                self.handle_syn(packet, addr).await;
            }
            CONNECT => {
                self.handle_connect(packet, addr).await;
            }
            v => {
                println!("unimplemented packed type: {}", v);
            }
        }
    }
    pub async fn run_task(self: Arc<Self>) {
        loop {
            let mut vec: Vec<u8> = vec![];
            let addr = match self.socket.recv_buf_from(&mut vec).await {
                Err(e) => {
                    error!("unable to recv: {}", e);
                    break;
                }
                Ok(v) => {
                    assert_eq!(vec.len(), v.0);
                    v.1
                }
            };
            let this = self.clone();
            tokio::spawn(async move {
                let data = vec;
                let packet = PRUDPV0Packet::new(&data[..]);

                let SocketAddr::V4(addr) = addr else {
                    unreachable!()
                };

                this.process_packet(packet, addr).await;
            });
        }
    }
    pub async fn new(param: ProxyStartupParam) -> Self {
        let socket = UdpSocket::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
            .await
            .expect("unable to bind socket");
        Self {
            socket,
            crypto: C::new(),
            connections: RwLock::new(HashMap::new()),
            param,
        }
    }
}
