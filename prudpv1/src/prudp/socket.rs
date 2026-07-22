use crate::prudp::packet::PacketOption::{
    ConnectionSignature, FragmentId, MaximumSubstreamId, SupportedFunctions,
};
use crate::prudp::packet::{PRUDPV1Header, PRUDPV1Packet};
use async_trait::async_trait;
use rc4::StreamCipher;
use rnex_prudp::socket_addr::PRUDPSockAddr;
use rnex_prudp::types_flags::TypesFlags;
use rnex_prudp::types_flags::flags::{ACK, HAS_SIZE, MULTI_ACK, NEED_ACK, RELIABLE};
use rnex_prudp::types_flags::types::{CONNECT, DATA, DISCONNECT, PING, SYN};
use rnex_prudp::virtual_port::VirtualPort;
use rnex_util::PID;
use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use tokio::spawn;
use tracing::error;
use tracing::{info, warn};
use v_byte_helpers::ReadExtensions;
use v_byte_helpers::little_endian::read_u16;

use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::{Instant, sleep};
// due to the way this is designed crashing the router thread causes deadlock, sorry ;-;
// (maybe i will fix that some day)

/// PRUDP Socket for accepting connections to then send and recieve data from those clients

pub struct CommonConnection {
    pub user_id: PID,
    pub socket_addr: PRUDPSockAddr,
    pub server_port: VirtualPort,
    session_id: u8,
}

struct InternalConnection<E: CryptoHandlerConnectionInstance> {
    common: Arc<CommonConnection>,
    connections: Weak<Mutex<BTreeMap<PRUDPSockAddr, Arc<InternalConnectionMutex<E>>>>>,
    reliable_server_counter: u16,
    reliable_client_counter: u16,
    // i'm a bit scared things might break if i remove this
    #[allow(dead_code)]
    supported_function_version: u32,
    #[deny(dead_code)]
    // maybe add connection id(need to see if its even needed)
    crypto_handler_instance: E,
    data_sender: Sender<Vec<u8>>,
    socket: Arc<UdpSocket>,
    packet_queue: HashMap<u16, PRUDPV1Packet>,
    last_packet_time: Instant,
    partial_packet: Vec<u8>,
    unacknowleged_packets: Vec<(Instant, PRUDPV1Packet)>,
}

struct InternalConnectionMutex<E: CryptoHandlerConnectionInstance>(Mutex<InternalConnection<E>>);

impl<E: CryptoHandlerConnectionInstance> AsRef<Mutex<InternalConnection<E>>>
    for InternalConnectionMutex<E>
{
    fn as_ref(&self) -> &Mutex<InternalConnection<E>> {
        &self.0
    }
}

impl<E: CryptoHandlerConnectionInstance> Deref for InternalConnectionMutex<E> {
    type Target = Mutex<InternalConnection<E>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: CryptoHandlerConnectionInstance> Deref for InternalConnection<E> {
    type Target = CommonConnection;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<E: CryptoHandlerConnectionInstance> InternalConnection<E> {
    /// gives back the next server packet sequence id which the client expects to send, incrementing it in the process
    fn next_server_count(&mut self) -> u16 {
        let prev_val = self.reliable_server_counter;
        let (val, _) = self.reliable_server_counter.overflowing_add(1);
        self.reliable_server_counter = val;

        prev_val
    }

    async fn close_connection(&mut self) {
        let mut packet = PRUDPV1Packet {
            header: PRUDPV1Header {
                sequence_id: self.next_server_count(),
                substream_id: 0,
                session_id: self.session_id,
                types_and_flags: TypesFlags::default().types(DISCONNECT),
                destination_port: self.common.socket_addr.virtual_port,
                source_port: self.server_port,
                ..Default::default()
            },
            payload: Vec::new(),
            options: vec![FragmentId(0)],
            ..Default::default()
        };

        // no need for encryption the, the payload is empty

        packet.set_sizes();

        self.crypto_handler_instance.sign_packet(&mut packet);

        self.send_raw_packet(&packet).await;

        self.delete_connection().await;
    }

    /// Sends a raw packet to a given client on the connection
    ///
    /// a raw packet is one which does not get processed any further(other than to send it
    /// off without buffering or anything),
    /// as such you need to make sure that
    /// the sizes are set correctly and so on
    #[inline]
    async fn send_raw_packet(&self, prudp_packet: &PRUDPV1Packet) {
        send_raw_prudp_to_sockaddr(&self.socket, self.socket_addr, prudp_packet).await;
    }

    async fn delete_connection(&self) {
        let Some(conns) = self.connections.upgrade() else {
            // this is fine as it implies the server has already quit, thus meaning that we dont
            // have to remove ourselves from the server
            return;
        };

        let mut conns = conns.lock().await;

        conns.remove(&self.socket_addr);

        // the connection will now drop as soon as we leave this due to no longer having a permanent
        // reference
    }
}

pub struct ExternalConnection<T: CryptoHandlerConnectionInstance> {
    sending: SendingConnection<T>,
    data_receiver: Receiver<Vec<u8>>,
}

pub struct SendingConnection<T: CryptoHandlerConnectionInstance> {
    common: Arc<CommonConnection>,
    internal: Weak<InternalConnectionMutex<T>>,
}

// we couldnt use the implementation the derive would generate here because that
// bakes in the assumption that all type parameters must be `Clone` for the struct
// we are deriving on to be `Clone` as well
impl<T: CryptoHandlerConnectionInstance> Clone for SendingConnection<T> {
    fn clone(&self) -> Self {
        Self {
            common: self.common.clone(),
            internal: self.internal.clone(),
        }
    }
}

pub struct CommonSocket {
    pub virtual_port: VirtualPort,
    _phantom_unconstructible: PhantomData<()>,
}

pub(super) struct InternalSocket<T: CryptoHandler> {
    common: Arc<CommonSocket>,
    socket: Arc<UdpSocket>,
    crypto_handler: T,
    // perf note: change the code to use RwLock here instead to avoid connections being able to block one another before the data is sent off.
    internal_connections: Arc<
        Mutex<BTreeMap<PRUDPSockAddr, Arc<InternalConnectionMutex<T::CryptoConnectionInstance>>>>,
    >,
    connection_establishment_data_sender: Mutex<Option<Sender<PRUDPV1Packet>>>,
    connection_sender: Sender<ExternalConnection<T::CryptoConnectionInstance>>,
}

pub struct ExternalSocket<T: CryptoHandler> {
    common: Arc<CommonSocket>,
    connection_receiver: Receiver<ExternalConnection<T::CryptoConnectionInstance>>,
    internal: Weak<InternalSocket<T>>,
}

impl<T: CryptoHandler> ExternalSocket<T> {
    pub async fn connect(
        &mut self,
        addr: PRUDPSockAddr,
    ) -> Option<ExternalConnection<T::CryptoConnectionInstance>> {
        let socket = self.internal.upgrade()?;

        socket.connect(addr).await;

        self.connection_receiver.recv().await
    }

    pub async fn accept(&mut self) -> Option<ExternalConnection<T::CryptoConnectionInstance>> {
        self.connection_receiver.recv().await
    }
}

impl<T: CryptoHandler> Deref for ExternalSocket<T> {
    type Target = CommonSocket;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<T: CryptoHandler> Deref for InternalSocket<T> {
    type Target = CommonSocket;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[async_trait]
pub(super) trait AnyInternalSocket:
    Send + Sync + Deref<Target = CommonSocket> + 'static
{
    async fn receive_packet(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet);
    async fn connect(&self, address: PRUDPSockAddr) -> Option<()>;
}

impl<E: CryptoHandlerConnectionInstance> InternalConnectionMutex<E> {
    async fn send_data_packet(&self, data: Vec<u8>) {
        let pieces = data.chunks(600);
        let max_piece = pieces.len() - 1;
        let mut piece_num = 1;
        let mut locked = self.lock().await;
        let packets: Vec<_> = pieces
            .enumerate()
            .map(|(i, piece)| {
                let mut packet = PRUDPV1Packet {
                    header: PRUDPV1Header {
                        sequence_id: locked.next_server_count(),
                        substream_id: 0,
                        session_id: locked.session_id,
                        types_and_flags: TypesFlags::default()
                            .types(DATA)
                            .flags(RELIABLE | NEED_ACK),
                        destination_port: locked.common.socket_addr.virtual_port,
                        source_port: locked.server_port,
                        ..Default::default()
                    },
                    payload: piece.to_owned(),
                    options: vec![FragmentId(if i == max_piece { 0 } else { piece_num })],
                    ..Default::default()
                };

                locked
                    .crypto_handler_instance
                    .encrypt_outgoing(0, &mut packet.payload[..]);

                packet.set_sizes();

                locked.crypto_handler_instance.sign_packet(&mut packet);

                piece_num += 1;
                packet
            })
            .collect();
        drop(locked);

        for packet in packets {
            let mut locked = self.lock().await;
            locked.send_raw_packet(&packet).await;

            locked.unacknowleged_packets.push((Instant::now(), packet));
            drop(locked);
            sleep(Duration::from_millis(16)).await;
        }
    }
}

async fn send_raw_prudp_to_sockaddr(
    udp_socket: &UdpSocket,
    dest: PRUDPSockAddr,
    packet: &PRUDPV1Packet,
) {
    let mut vec = Vec::new();

    packet
        .write_to(&mut vec)
        .expect("somehow failed to convert backet to bytes");

    udp_socket
        .send_to(&vec, dest.regular_socket_addr)
        .await
        .expect("failed to send data back");
}

impl<T: CryptoHandler> InternalSocket<T> {
    async fn get_connection(
        &self,
        addr: PRUDPSockAddr,
    ) -> Option<Arc<InternalConnectionMutex<T::CryptoConnectionInstance>>> {
        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&addr) else {
            error!("tried to send data on inactive connection!");
            return None;
        };

        let conn = conn.clone();
        drop(connections);

        Some(conn)
    }

    /// sends a raw packet to a specific prudp socket address
    ///
    /// a raw packet is a packet is a packet which wont get processed any further,
    /// sizes signatures etc need to be set before using this function
    async fn send_packet_unbuffered(&self, dest: PRUDPSockAddr, packet: &PRUDPV1Packet) {
        send_raw_prudp_to_sockaddr(&self.socket, dest, packet).await
    }

    async fn handle_syn(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        info!("got syn");

        let mut response = packet.base_response_packet();

        response.header.types_and_flags.set_types(SYN);
        response.header.types_and_flags.set_flag(ACK);
        response.header.types_and_flags.set_flag(HAS_SIZE);

        let signature = address.calculate_connection_signature();

        response.options.push(ConnectionSignature(signature));

        // todo: refactor this to be more readable(low priority cause it doesnt change anything api wise)
        for options in &packet.options {
            match options {
                SupportedFunctions(functions) => {
                    response.options.push(SupportedFunctions(*functions & 0xFF))
                }
                MaximumSubstreamId(max_substream) => {
                    response.options.push(MaximumSubstreamId(*max_substream))
                }
                _ => { /* ??? */ }
            }
        }

        response.set_sizes();

        self.crypto_handler.sign_pre_handshake(&mut response);

        //println!("got syn: {:?}", response);

        self.send_packet_unbuffered(address, &response).await;
    }

    async fn connection_thread(
        connection: Weak<InternalConnectionMutex<T::CryptoConnectionInstance>>,
    ) {
        //todo: handle stuff like resending packets if they arent acknowledged in here

        while let Some(conn) = connection.upgrade() {
            let mut conn = conn.lock().await;

            if conn.last_packet_time < (Instant::now() - Duration::from_secs(5)) {
                conn.send_raw_packet(&PRUDPV1Packet {
                    header: PRUDPV1Header {
                        sequence_id: 0,
                        substream_id: 0,
                        session_id: 0,
                        types_and_flags: TypesFlags::default().types(PING).flags(NEED_ACK),
                        destination_port: conn.common.socket_addr.virtual_port,
                        source_port: conn.server_port,
                        ..Default::default()
                    },
                    payload: Vec::new(),
                    options: vec![],
                    ..Default::default()
                })
                .await;
            }

            if conn.last_packet_time < (Instant::now() - Duration::from_secs(30)) {
                conn.close_connection().await;
            }

            for (send_time, packet) in &conn.unacknowleged_packets {
                if *send_time < (Instant::now() - Duration::from_millis(3000)) {
                    warn!(
                        "failed to resend packet 5 times and never got response, destroying connection"
                    );
                    conn.close_connection().await;
                    break;
                }
                if *send_time < (Instant::now() - Duration::from_millis(500)) {
                    info!("unacknowledged packet sat arround for more than 500 ms, resending");
                    conn.send_raw_packet(packet).await;
                }
            }

            drop(conn);

            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn create_connection(
        &self,
        crypto_handler_instance: T::CryptoConnectionInstance,
        socket_addr: PRUDPSockAddr,
        session_id: u8,
        is_instantiator: bool,
        supported_function_version: u32,
    ) {
        let common = Arc::new(CommonConnection {
            user_id: crypto_handler_instance.get_user_id(),
            socket_addr,
            session_id,
            server_port: self.virtual_port,
        });

        let (data_sender_from_client, data_receiver_from_client) = channel(16);

        let internal = InternalConnection {
            common: common.clone(),
            crypto_handler_instance,
            connections: Arc::downgrade(&self.internal_connections),
            reliable_client_counter: if is_instantiator { 1 } else { 2 },
            reliable_server_counter: if is_instantiator { 2 } else { 1 },
            data_sender: data_sender_from_client,
            socket: self.socket.clone(),
            packet_queue: Default::default(),
            last_packet_time: Instant::now(),
            unacknowleged_packets: Vec::new(),
            partial_packet: Vec::new(),
            supported_function_version,
        };

        let internal = Arc::new(InternalConnectionMutex(Mutex::new(internal)));

        let dyn_internal = internal.clone();

        let external = ExternalConnection {
            sending: SendingConnection {
                common,
                internal: Arc::downgrade(&dyn_internal),
            },
            data_receiver: data_receiver_from_client,
        };

        let mut connections = self.internal_connections.lock().await;

        connections.insert(socket_addr, internal.clone());

        drop(connections);

        tokio::spawn(Self::connection_thread(Arc::downgrade(&internal)));

        self.connection_sender
            .send(external)
            .await
            .expect("connection to external socket lost");
    }

    async fn handle_connect(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        info!("got connect");
        let Some(MaximumSubstreamId(max_substream)) = packet
            .options
            .iter()
            .find(|v| matches!(v, MaximumSubstreamId(_)))
        else {
            return;
        };

        let remote_signature = address.calculate_connection_signature();

        let Some(ConnectionSignature(own_signature)) = packet
            .options
            .iter()
            .find(|p| matches!(p, ConnectionSignature(_)))
        else {
            error!("didnt get connection signature from client");
            return;
        };

        let session_id = packet.header.session_id;

        let Some((return_data, crypto)) = self.crypto_handler.instantiate(
            remote_signature,
            *own_signature,
            &packet.payload,
            1 + *max_substream,
        ) else {
            error!("someone attempted to connect with invalid data");
            return;
        };

        let mut response = packet.base_response_packet();
        response.header.types_and_flags.set_types(CONNECT);
        response.header.types_and_flags.set_flag(ACK);
        response.header.types_and_flags.set_flag(HAS_SIZE);

        response.header.session_id = session_id;
        response.header.sequence_id = 1;

        response.payload = return_data;

        //let remote_signature = address.calculate_connection_signature();

        response
            .options
            .push(ConnectionSignature(Default::default()));

        let mut functions: u32 = 0;

        for option in &packet.options {
            match option {
                MaximumSubstreamId(max_substream) => {
                    response.options.push(MaximumSubstreamId(*max_substream))
                }
                SupportedFunctions(funcs) => {
                    functions = *funcs & 0xFF;
                    response.options.push(SupportedFunctions(*funcs & 0xFF));
                }
                _ => { /* ? */ }
            }
        }

        response.set_sizes();

        crypto.sign_connect(&mut response);

        //println!("connect out: {:?}", response);

        self.create_connection(crypto, address, session_id, false, functions)
            .await;

        self.send_packet_unbuffered(address, &response).await;
    }

    async fn handle_data(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        info!("got data");

        if packet.header.types_and_flags.get_flags() & (NEED_ACK | RELIABLE)
            != (NEED_ACK | RELIABLE)
        {
            error!("invalid or unimplemented packet flags");
        }

        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else {
            error!("tried to send data on inactive connection!");
            return;
        };

        let conn = conn.clone();
        drop(connections);

        println!("recieved packed id: {}", packet.header.sequence_id);

        let mut conn = conn.lock().await;

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, &response).await;

        conn.packet_queue.insert(packet.header.sequence_id, packet);

        let mut counter = conn.reliable_client_counter;

        while let Some(mut packet) = conn.packet_queue.remove(&counter) {
            conn.crypto_handler_instance
                .decrypt_incoming(packet.header.substream_id, &mut packet.payload[..]);

            conn.partial_packet
	                .extend_from_slice(&mut packet.payload[..]);

            conn.reliable_client_counter = conn.reliable_client_counter.overflowing_add(1).0;
            counter = conn.reliable_client_counter;
            if packet.options.iter().any(|v| {
                if let FragmentId(f) = v {
                    *f != 0
                } else {
                    false
                }
            }) {
                println!("handeling fragmented packet");
                continue;
            }

            let packet = std::mem::take(&mut conn.partial_packet);

            conn.data_sender.send(packet).await.ok();
        }
    }

    async fn handle_ping(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else {
            error!("tried to send data on inactive connection!");
            return;
        };
        let conn = conn.clone();
        drop(connections);

        let conn = conn.lock().await;

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, &response).await;
    }

    async fn handle_disconnect(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else {
            error!("tried to send data on inactive connection!");
            return;
        };
        let conn = conn.clone();
        drop(connections);

        let conn = conn.lock().await;

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, &response).await;
        self.send_packet_unbuffered(address, &response).await;
        self.send_packet_unbuffered(address, &response).await;

        conn.delete_connection().await;
        //self.internal_connections.lock().await;
    }
}

#[async_trait]
impl<T: CryptoHandler> AnyInternalSocket for InternalSocket<T> {
    async fn receive_packet(&self, address: PRUDPSockAddr, packet: PRUDPV1Packet) {
        // todo: handle acks and resending

        if let Some(conn) = self.get_connection(address).await {
            let mut conn = conn.lock().await;

            // reset timeout
            conn.last_packet_time = Instant::now();
        }

        if (packet.header.types_and_flags.get_flags() & ACK) != 0 {
            info!("got ack");

            if packet.header.types_and_flags.get_types() == SYN
                || packet.header.types_and_flags.get_types() == CONNECT
            {
                if packet.header.types_and_flags.get_types() == SYN {
                    println!("Syn: {:?}", packet);
                }

                if packet.header.types_and_flags.get_types() == CONNECT {
                    println!("Connect: {:?}", packet);
                }

                let sender = self.connection_establishment_data_sender.lock().await;
                info!("redirecting ack to active connection establishment code");

                if let Some(conn) = sender.as_ref() {
                    if let Err(e) = conn.send(packet).await {
                        error!(
                            "error whilst sending data to connection establishment: {}",
                            e
                        );
                    }
                } else {
                    error!("got connection response without the active reciever being present");
                }
                return;
            }

            info!("got ack");

            if let Some(conn) = self.get_connection(address).await {
                let mut conn = conn.lock().await;

                // remove the packet whose sequence id matches the ack packet
                // or in other words keep all of those which dont match the sequence id
                conn.unacknowleged_packets
                    .retain_mut(|v| packet.header.sequence_id != v.1.header.sequence_id);
            } else {
                error!("non connection acknowledgement packet on nonexistent connection...")
            }

            return;
        }

        if (packet.header.types_and_flags.get_flags() & MULTI_ACK) != 0 {
            if let Some(conn) = self.get_connection(address).await {
                let conn = &**conn;
                let mut conn = conn.lock().await;

                if packet.header.substream_id == 1 {
                    let mut collected_ids: Vec<u16> = Vec::new();
                    let mut cursor = Cursor::new(&packet.payload);

                    let Ok(_substream_id): Result<u8, _> = cursor.read_le_struct() else {
                        error!("invalid data whilst reading new version agregate acknowledgement");
                        return;
                    };
                    let Ok(additional_sequence_ids): Result<u8, _> = cursor.read_le_struct() else {
                        error!("invalid data whilst reading new version agregate acknowledgement");
                        return;
                    };
                    let Ok(sequence_id): Result<u16, _> = cursor.read_le_struct() else {
                        error!("invalid data whilst reading new version agregate acknowledgement");
                        return;
                    };
                    for _ in 0..additional_sequence_ids {
                        let Ok(additional_sequence_id): Result<u16, _> = cursor.read_le_struct()
                        else {
                            error!(
                                "invalid data whilst reading new version agregate acknowledgement"
                            );
                            return;
                        };
                        collected_ids.push(additional_sequence_id);
                    }

                    conn.unacknowleged_packets.retain(|(_, up)| {
                        !(collected_ids.iter().any(|id| up.header.sequence_id == *id)
                            || up.header.sequence_id <= sequence_id)
                    });
                } else {
                    let mut collected_ids: Vec<u16> = Vec::new();
                    let mut cursor = Cursor::new(&packet.payload);

                    while let Ok(v) = read_u16(&mut cursor) {
                        collected_ids.push(v);
                    }

                    conn.unacknowleged_packets.retain(|(_, up)| {
                        !(collected_ids.iter().any(|id| up.header.sequence_id == *id)
                            || up.header.sequence_id <= packet.header.sequence_id)
                    });
                }
            } else {
                error!("non connection acknowledgement packet on nonexistent connection...")
            }
            return;
        }

        match packet.header.types_and_flags.get_types() {
            SYN => self.handle_syn(address, packet).await,
            CONNECT => self.handle_connect(address, packet).await,
            DATA => self.handle_data(address, packet).await,
            DISCONNECT => self.handle_disconnect(address, packet).await,
            PING => self.handle_ping(address, packet).await,
            _ => {
                error!(
                    "unimplemented packet type: {}",
                    packet.header.types_and_flags.get_types()
                )
            }
        }
    }

    async fn connect(&self, address: PRUDPSockAddr) -> Option<()> {
        let (send, mut recv) = channel(10);

        let mut sender = self.connection_establishment_data_sender.lock().await;
        *sender = Some(send);
        drop(sender);

        let remote_signature = address.calculate_connection_signature();

        let mut packet = PRUDPV1Packet {
            header: PRUDPV1Header {
                source_port: self.virtual_port,
                destination_port: address.virtual_port,
                types_and_flags: TypesFlags::default().types(SYN).flags(NEED_ACK),
                ..Default::default()
            },
            options: vec![
                SupportedFunctions(0x104),
                MaximumSubstreamId(0),
                ConnectionSignature(remote_signature),
            ],
            ..Default::default()
        };

        packet.set_sizes();

        self.send_packet_unbuffered(address, &packet).await;

        let Some(syn_ack_packet) = recv.recv().await else {
            error!("what");
            return None;
        };

        let Some(ConnectionSignature(own_signature)) = syn_ack_packet
            .options
            .iter()
            .find(|p| matches!(p, ConnectionSignature(_)))
        else {
            error!("didnt get connection signature from remote partner");
            return None;
        };

        let mut packet = PRUDPV1Packet {
            header: PRUDPV1Header {
                source_port: self.virtual_port,
                destination_port: address.virtual_port,
                types_and_flags: TypesFlags::default().types(CONNECT).flags(NEED_ACK),
                ..Default::default()
            },
            options: vec![
                SupportedFunctions(0x04),
                MaximumSubstreamId(0),
                ConnectionSignature(remote_signature),
            ],
            ..Default::default()
        };

        packet.set_sizes();

        self.send_packet_unbuffered(address, &packet).await;

        let Some(_connect_ack_packet) = recv.recv().await else {
            error!("what");
            return None;
        };

        let (_, crypt) =
            self.crypto_handler
                .instantiate(remote_signature, *own_signature, &[], 1)?;

        //todo: make this work for secure servers as well
        self.create_connection(crypt, address, 0, true, 4).await;

        Some(())
    }
}

pub(super) fn new_socket_pair<T: CryptoHandler>(
    virtual_port: VirtualPort,
    encryption: T,
    socket: Arc<UdpSocket>,
) -> (Arc<InternalSocket<T>>, ExternalSocket<T>) {
    let common = Arc::new(CommonSocket {
        virtual_port,
        _phantom_unconstructible: Default::default(),
    });

    let (connection_send, connection_recv) = channel(16);

    let internal = Arc::new(InternalSocket {
        common: common.clone(),
        connection_sender: connection_send,
        crypto_handler: encryption,
        internal_connections: Default::default(),
        connection_establishment_data_sender: Default::default(),
        socket,
    });

    let dyn_internal = internal.clone();

    let external = ExternalSocket {
        common,
        connection_receiver: connection_recv,
        internal: Arc::downgrade(&dyn_internal),
    };

    (internal, external)
}

pub trait CryptoHandlerConnectionInstance: Send + Sync + 'static {
    type Encryption: StreamCipher + Send;

    fn decrypt_incoming(&mut self, substream: u8, data: &mut [u8]);
    fn encrypt_outgoing(&mut self, substream: u8, data: &mut [u8]);

    fn get_user_id(&self) -> i32;
    fn sign_connect(&self, packet: &mut PRUDPV1Packet);
    fn sign_packet(&self, packet: &mut PRUDPV1Packet);
    fn verify_packet(&self, packet: &PRUDPV1Packet) -> bool;
}

pub trait CryptoHandler: Send + Sync + 'static {
    type CryptoConnectionInstance: CryptoHandlerConnectionInstance;

    fn instantiate(
        &self,
        remote_signature: [u8; 16],
        own_signature: [u8; 16],
        _: &[u8],
        substream_count: u8,
    ) -> Option<(Vec<u8>, Self::CryptoConnectionInstance)>;

    fn sign_pre_handshake(&self, packet: &mut PRUDPV1Packet);
}

impl<T: CryptoHandlerConnectionInstance> Deref for ExternalConnection<T> {
    type Target = SendingConnection<T>;
    fn deref(&self) -> &Self::Target {
        &self.sending
    }
}

impl<T: CryptoHandlerConnectionInstance> Deref for SendingConnection<T> {
    type Target = CommonConnection;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<E: CryptoHandlerConnectionInstance> ExternalConnection<E> {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.data_receiver.recv().await
    }
    //todo: make this an actual result instead of an option

    pub fn duplicate_sender(&self) -> SendingConnection<E> {
        self.sending.clone()
    }
}

impl<E: CryptoHandlerConnectionInstance> SendingConnection<E> {
    pub async fn send(&self, data: Vec<u8>) -> Option<()> {
        let internal = self.internal.upgrade()?;
        spawn(async move {
            internal.send_data_packet(data).await;
        });
        Some(())
    }

    pub async fn close_connection(&self) {
        let Some(internal) = self.internal.upgrade() else {
            return;
        };

        let mut internal = internal.lock().await;

        internal.close_connection().await;
    }
}

impl<E: CryptoHandlerConnectionInstance> Drop for InternalConnection<E> {
    fn drop(&mut self) {
        println!("s2s connection disconnected");
    }
}

impl Drop for CommonConnection {
    fn drop(&mut self) {
        println!("client disconnected");
    }
}
