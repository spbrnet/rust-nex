use std::{
    collections::HashMap,
    hash::Hash,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        Arc, LazyLock, Weak,
        atomic::{AtomicBool, AtomicU32},
    },
    time::Duration,
};

use log::{error, info, warn};
use proxy_common::{ProxyStartupParam, new_backend_connection};
use rnex_core::{
    executables::common::{OWN_IP_PRIVATE, SERVER_PORT},
    prudp::{
        socket_addr::PRUDPSockAddr,
        types_flags::{
            TypesFlags,
            flags::{ACK, HAS_SIZE, NEED_ACK, RELIABLE},
            types::{CONNECT, DATA, DISCONNECT, PING, SYN},
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
    time::{Instant, sleep},
};

use crate::{
    crypto::{Crypto, CryptoInstance},
    packet::{
        PRUDPV0Header, PRUDPV0Packet, new_connect_packet, new_data_packet, new_disconnect_packet,
        new_ping_packet, new_syn_packet, precalc_size,
    },
};

pub struct InternalConnection<C: CryptoInstance> {
    last_action: Instant,
    crypto_instance: C,
    server_packet_counter: u16,
    client_packet_counter: u16,
    unacknowledged_packets: HashMap<u16, Arc<Vec<u8>>>,
    packet_queue: HashMap<u16, (Instant, PRUDPV0Packet<Vec<u8>>)>,
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
    connections: RwLock<HashMap<(PRUDPSockAddr, u8), Arc<Connection<C::Instance>>>>,
}

impl<C: Crypto> Server<C> {
    async fn send_data_packet(self: Arc<Self>, conn: Arc<Connection<C::Instance>>, data: &[u8]) {
        /*let type_flags = TypesFlags::default().types(DATA).flags(HAS_SIZE | NEED_ACK);
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

        packet.checksum_mut().expect("packet malformed in creation") =
            self.crypto.calculate_checksum(
                packet
                    .checksummed_data()
                    .expect("packet malformed in creation"),
            );*/
        let mut inner = conn.inner.lock().await;
        let pieces = data.chunks(700);
        let max_piece = pieces.len() - 1;
        let mut frag_num = 1;
        for (i, piece) in pieces.enumerate() {
            let seq = inner.server_packet_counter;
            let packet = new_data_packet(
                NEED_ACK | RELIABLE,
                (&self).param.virtual_port,
                conn.addr.virtual_port,
                piece,
                inner.server_packet_counter,
                conn.session_id,
                if i == max_piece { 0 } else { frag_num },
                &mut inner.crypto_instance,
                &(&self).crypto,
            );
            inner.server_packet_counter += 1;

            let packet = Arc::new(packet);
            let packet_ref = Arc::downgrade(&packet);

            inner.unacknowledged_packets.insert(seq, packet);

            let conn = Arc::downgrade(&conn);
            let this = Arc::downgrade(&self);

            spawn(async move {
                sleep(Duration::from_millis(i as u64 * 16)).await;
                for n in 0..5 {
                    let Some(data) = packet_ref.upgrade() else {
                        return;
                    };
                    let Some(conn) = conn.upgrade() else {
                        return;
                    };
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    info!("send attempt {}", n);

                    this.socket
                        .send_to(&data, conn.addr.regular_socket_addr)
                        .await;

                    break;
                }
            });
            frag_num += 1;
        }
        drop(inner);
    }
    async fn connection_thread(
        self: Arc<Self>,
        conn: Weak<Connection<C::Instance>>,
        mut recv: SplittableBufferConnection,
    ) {
        while let Some(data) = recv.recv().await {
            let Some(conn) = conn.upgrade() else { break };
            if &data[..] == &[0, 0, 0, 0, 0] {
                info!("got keepalive");
                continue;
            }
            info!("got data from server: {:?}", data);
            self.clone().send_data_packet(conn.clone(), &data).await;
        }
    }
    async fn timeout_thread(self: Arc<Self>, conn: Weak<Connection<C::Instance>>) {
        loop {
            let Some(conn) = conn.upgrade() else { break };
            sleep(Duration::from_secs(3)).await;
            let mut inner = conn.inner.lock().await;

            if (Instant::now() - inner.last_action).as_secs() > 5 {
                warn!("connection exceeded silence limit, sending ping");
                let packet = new_ping_packet(
                    NEED_ACK,
                    self.param.virtual_port,
                    conn.addr.virtual_port,
                    0,
                    conn.session_id,
                    &mut inner.crypto_instance,
                    &self.crypto,
                );

                self.socket
                    .send_to(&packet, conn.addr.regular_socket_addr)
                    .await;
            }

            if (Instant::now() - inner.last_action).as_secs() > 15 {
                warn!("client timed out...");

                let packet = new_disconnect_packet(
                    NEED_ACK,
                    self.param.virtual_port,
                    conn.addr.virtual_port,
                    0,
                    conn.session_id,
                    &mut inner.crypto_instance,
                    &self.crypto,
                );

                self.socket
                    .send_to(&packet, conn.addr.regular_socket_addr)
                    .await;
                self.socket
                    .send_to(&packet, conn.addr.regular_socket_addr)
                    .await;
                self.socket
                    .send_to(&packet, conn.addr.regular_socket_addr)
                    .await;
                drop(inner);

                let mut conns = self.connections.write().await;
                conns.remove(&(conn.addr, conn.session_id));
                drop(conns);
                break;
            }

            drop(inner);
        }
    }
    async fn handle_syn(self: Arc<Self>, packet: PRUDPV0Packet<Vec<u8>>, addr: PRUDPSockAddr) {
        info!("got syn");
        let header = packet.header().unwrap();

        let signat = addr.calculate_connection_signature();
        let signat = [signat[0], signat[1], signat[2], signat[3]];

        let packet = new_syn_packet(ACK, header.destination, header.source, signat, &self.crypto);
        self.socket.send_to(&packet, addr.regular_socket_addr).await;
    }
    async fn handle_connect(self: Arc<Self>, packet: PRUDPV0Packet<Vec<u8>>, addr: PRUDPSockAddr) {
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
        let remote_signat = addr.calculate_connection_signature();
        let remote_signat = [
            remote_signat[0],
            remote_signat[1],
            remote_signat[2],
            remote_signat[3],
        ];

        let Some((ci, data)) = self.crypto.instantiate(&data, self_signat, remote_signat) else {
            warn!("unable to instantiate crypto instance");
            return;
        };

        let pid = ci.get_user_id();
        println!("user with pid {} is connecting", pid);
        let buf_conn = new_backend_connection(&self.param, addr, pid).await;
        let Some(buf_conn) = buf_conn else {
            error!("unable to connect to backend");
            return;
        };

        let header = packet.header().expect("header should be validated by now");

        let conn = Arc::new(Connection {
            target: buf_conn.duplicate_sender(),
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
                packet_queue: HashMap::new(),
            }),
        });

        let mut conns = self.connections.write().await;
        if conns.contains_key(&(addr, header.session_id)) {
            error!("client already connected but tried to connect again");
        }
        conns.insert((addr, header.session_id), conn.clone());
        drop(conns);

        spawn({
            let this = self.clone();
            let conn = Arc::downgrade(&conn);
            this.connection_thread(conn, buf_conn)
        });
        spawn({
            let this = self.clone();
            let conn = Arc::downgrade(&conn);
            this.timeout_thread(conn)
        });

        let packet = new_connect_packet(
            ACK,
            header.destination,
            header.source,
            self_signat,
            remote_signat,
            packet.header().unwrap().session_id,
            &data,
            &self.crypto,
        );

        info!("sending back connection accept");
        self.socket.send_to(&packet, addr.regular_socket_addr).await;
    }
    async fn handle_data(self: Arc<Self>, mut packet: PRUDPV0Packet<Vec<u8>>, addr: PRUDPSockAddr) {
        let Some(frag_id) = packet.fragment_id() else {
            warn!("invalid packet from: {:?}", addr);
            return;
        };
        let Some(header) = packet.header() else {
            warn!("invalid packet from: {:?}", addr);
            return;
        };

        let Some(res) = self.get_connection((addr, header.session_id)).await else {
            warn!("data packet on inactive connection from: {:?}", addr);
            return;
        };
        info!("frag: {}", frag_id);
        let mut conn = res.inner.lock().await;
        let ack = new_data_packet(
            ACK,
            self.param.virtual_port,
            res.addr.virtual_port,
            &[],
            header.sequence_id,
            header.session_id,
            *frag_id,
            &mut conn.crypto_instance,
            &self.crypto,
        );
        self.socket.send_to(&ack, addr.regular_socket_addr).await;
        conn.packet_queue.insert(
            packet.header().unwrap().sequence_id,
            (Instant::now(), packet),
        );
        while let Some((_, mut packet)) = {
            let ctr = conn.client_packet_counter;
            let packet = conn.packet_queue.remove(&ctr);
            packet
        } {
            info!("processing packet: {}", conn.client_packet_counter);
            let Some(payload) = packet.payload_mut() else {
                //todo: at this point the stream would have been broken, we should probably disconnect the client
                warn!("invalid packet from: {:?}", addr);
                return;
            };

            conn.crypto_instance.decrypt_incoming(payload);

            res.target.send(payload.to_owned()).await;
            conn.client_packet_counter += 1;
        }
        info!("finished handeling packets, dropping inner connection");
        drop(conn);
    }

    async fn handle_ping(self: Arc<Self>, mut packet: PRUDPV0Packet<Vec<u8>>, addr: PRUDPSockAddr) {
        info!("got ping");
        let header = packet.header().unwrap();

        let Some(conn) = self.get_connection((addr, header.session_id)).await else {
            warn!("ping on inactive connection: {:?}", addr);
            return;
        };
        let mut inner = conn.inner.lock().await;
        let packet = new_ping_packet(
            ACK,
            self.param.virtual_port,
            addr.virtual_port,
            header.sequence_id,
            header.session_id,
            &mut inner.crypto_instance,
            &self.crypto,
        );
        drop(inner);

        self.socket.send_to(&packet, addr.regular_socket_addr).await;
    }
    async fn handle_disconnect(
        self: Arc<Self>,
        mut packet: PRUDPV0Packet<Vec<u8>>,
        addr: PRUDPSockAddr,
    ) {
        info!("got disconnect");
        let header = packet.header().unwrap();

        let Some(conn) = self.get_connection((addr, header.session_id)).await else {
            warn!("ping on inactive connection: {:?}", addr);
            return;
        };
        let mut inner = conn.inner.lock().await;
        let packet = new_disconnect_packet(
            ACK,
            self.param.virtual_port,
            addr.virtual_port,
            header.sequence_id,
            header.session_id,
            &mut inner.crypto_instance,
            &self.crypto,
        );
        drop(inner);

        let mut conns = self.connections.write().await;
        conns.remove(&(addr, header.session_id));
        drop(conns);

        self.socket.send_to(&packet, addr.regular_socket_addr).await;
        self.socket.send_to(&packet, addr.regular_socket_addr).await;
        self.socket.send_to(&packet, addr.regular_socket_addr).await;
    }
    async fn get_connection(
        &self,
        addr: (PRUDPSockAddr, u8),
    ) -> Option<Arc<Connection<C::Instance>>> {
        let rd = self.connections.read().await;
        let res = rd.get(&addr).cloned();
        drop(rd);
        res
    }

    async fn process_packet(self: Arc<Self>, packet: PRUDPV0Packet<Vec<u8>>, addr: SocketAddrV4) {
        if !packet.check_checksum(&self.crypto) {
            warn!("invalid checksum from: {}", addr);
            return;
        }

        let Some(header) = packet.header() else {
            warn!("malformatted packet from: {}", addr);
            return;
        };

        info!("len: {}", packet.0.len());

        let addr = PRUDPSockAddr::new(SocketAddr::V4(addr), header.source);

        if let Some(conn) = self.get_connection((addr, header.session_id)).await {
            let mut inner = conn.inner.lock().await;
            inner.last_action = Instant::now();
            drop(inner);
        };
        if header.type_flags.get_flags() & ACK != 0 {
            info!("got ack(acks are ignored for now)");
            return;
        }
        println!("{:?}", header);
        match header.type_flags.get_types() {
            SYN => {
                self.handle_syn(packet, addr).await;
            }
            CONNECT => {
                self.handle_connect(packet, addr).await;
            }
            DATA => {
                self.handle_data(packet, addr).await;
            }
            PING => {
                self.handle_ping(packet, addr).await;
            }
            DISCONNECT => {
                self.handle_disconnect(packet, addr).await;
            }
            v => {
                println!("unimplemented packed type: {}", v);
            }
        }
    }
    pub async fn run_task(self: Arc<Self>) {
        loop {
            let mut vec: Vec<u8> = vec![0u8; 65507];
            let (len, addr) = match self.socket.recv_from(&mut vec).await {
                Err(e) => {
                    error!("unable to recv: {}", e);
                    break;
                }
                Ok(v) => v,
            };
            let this = self.clone();
            tokio::spawn(async move {
                let mut data = vec;
                data.resize(len, 0);
                let packet = PRUDPV0Packet::new(data);

                let SocketAddr::V4(addr) = addr else {
                    unreachable!()
                };

                this.process_packet(packet, addr).await;
            });
        }
    }
    pub async fn new(param: ProxyStartupParam) -> Self {
        let socket = UdpSocket::bind(param.self_private)
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
