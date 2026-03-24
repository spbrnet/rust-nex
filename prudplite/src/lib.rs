pub mod crypto;
mod packet;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use crate::{
    crypto::{Crypto, insecure::Insecure, secure::Secure},
    packet::{LiteHeader, LitePacket, PacketSpecificData, StreamTypes, create_packet_from},
};
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use proxy_common::{ProxyStartupParam, new_backend_connection};
use rnex_core::{
    PID,
    prudp::{
        socket_addr::PRUDPSockAddr,
        types_flags::{
            TypesFlags,
            flags::{ACK, NEED_ACK, RELIABLE},
            types::{CONNECT, DATA, DISCONNECT, SYN},
        },
        virtual_port::VirtualPort,
    },
    util::SplittableBufferConnection,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Bytes, Message, client::IntoClientRequest, http::header::ACCESS_CONTROL_REQUEST_METHOD,
    },
};

struct ConnectionState {
    param: Arc<ProxyStartupParam>,
    active: bool,
    websocket: WebSocketStream<TcpStream>,
    pid: PID,
    backend_conn: SplittableBufferConnection,
    addr: PRUDPSockAddr,
    incoming_reliable: HashMap<u16, LitePacket<Bytes>>,
    client_reliable_counter: u16,
    server_reliable_counter: u16,
}

impl ConnectionState {
    pub async fn handle_incoming_prudp(&mut self, packet: LitePacket<Bytes>, sorted: bool) {
        let Some(header) = packet.header() else {
            warn!("invalid data on connection");
            return;
        };

        if (header.types_flags.get_flags() & NEED_ACK) != 0 {
            let data = create_packet_from(
                LiteHeader {
                    stream_types: StreamTypes::new(
                        self.param.virtual_port.get_stream_type(),
                        self.addr.virtual_port.get_stream_type(),
                    ),
                    source_port: self.param.virtual_port.get_port_number(),
                    destination_port: self.addr.virtual_port.get_port_number(),
                    fragment_id: header.fragment_id,
                    types_flags: TypesFlags::default()
                        .types(header.types_flags.get_types())
                        .flags(ACK),
                    sequence_id: header.sequence_id,
                    ..Default::default()
                },
                &[],
                &[],
            );
            let data: Bytes = data.into();
            if header.types_flags.get_types() == DISCONNECT {
                self.websocket.send(Message::Binary(data.clone())).await;
                self.websocket.send(Message::Binary(data.clone())).await;
            }
            self.websocket.send(Message::Binary(data)).await;
        }

        if (header.types_flags.get_flags() & ACK) != 0 {
            // we can just safely ignore acks, we ARE sending over tcp after all already guarantees that our packets will arrive
            // we can however not guarantee the order of incoming client packets so we should still take care of that
            // (the client might be doing some funny things which we dont know of)
            return;
        }

        if (header.types_flags.get_flags() & RELIABLE != 0) & !sorted {
            self.incoming_reliable.insert(header.sequence_id, packet);
            if self.incoming_reliable.len() > 5 {
                self.active = false;
                warn!("client is spamming out of order reliable packets, throwing out");
            }
            return;
        }

        match header.types_flags.get_types() {
            DATA => {
                if header.fragment_id != 0 {
                    warn!("fragmented packets arent yet supported");
                    return;
                }

                let Some(payload) = packet.payload() else {
                    return;
                };
                self.backend_conn.send(payload.into()).await;
            }
            PING => {}
            v => {
                info!("unimplemented packet type: {}", v);
            }
        }
    }
    pub async fn process_reliable(&mut self) {
        while let Some(v) = self.incoming_reliable.remove(&self.client_reliable_counter) {
            self.handle_incoming_prudp(v, true).await;
            self.client_reliable_counter += 1;
        }
    }
    pub async fn handle_connection(&mut self) {
        while self.active {
            tokio::select! {
                v = self.websocket.next() => {
                    match v {
                        Some(Ok(Message::Binary(v))) => {
                            self.handle_incoming_prudp(LitePacket::new(v), false).await;
                        }
                        _ => {
                            info!("client disconnected or errored out");
                            return;
                        }
                    }
                }
                v = self.backend_conn.recv() => {

                }
            }
        }
    }
}

pub async fn websocket_thread_unconnected<C: Crypto>(
    param: Arc<ProxyStartupParam>,
    crypto: Arc<C>,
    conn: TcpStream,
    addr: SocketAddr,
) {
    let mut websocket = match tokio_tungstenite::accept_async(conn).await {
        Ok(v) => v,
        Err(e) => {
            error!("error accepting websocket connection: {}", e);
            return;
        }
    };

    while let Some(Ok(v)) = websocket.next().await {
        match v {
            Message::Binary(b) => {
                let packet = LitePacket::new(b);

                let Some(header) = packet.header() else {
                    error!("got malformed message, disconnecting");
                    return;
                };

                match header.types_flags.get_types() {
                    SYN => {
                        let Some(supported) = packet.packet_specific_iter() else {
                            error!("got malformed message, disconnecting");
                            return;
                        };

                        let Some(PacketSpecificData::SupportedFunctions(s)) = supported
                            .into_iter()
                            .find(|v| matches!(v, PacketSpecificData::SupportedFunctions(_)))
                        else {
                            error!("got malformed message, disconnecting");
                            return;
                        };

                        let data = create_packet_from(
                            LiteHeader {
                                destination_port: header.source_port,
                                source_port: param.virtual_port.get_port_number(),
                                stream_types: StreamTypes::new(
                                    param.virtual_port.get_stream_type(),
                                    header.stream_types.source(),
                                ),
                                fragment_id: 0,
                                sequence_id: 0,
                                types_flags: TypesFlags::default().types(SYN).flags(ACK),
                                ..Default::default()
                            },
                            &[
                                PacketSpecificData::SupportedFunctions(s & 0xFF),
                                PacketSpecificData::ConnectionSignature([0; 16]),
                            ],
                            &[],
                        );
                        websocket.send(Message::Binary(data.into())).await;
                    }
                    CONNECT => {
                        let Some(supported) = packet.packet_specific_iter() else {
                            error!("got malformed message, disconnecting");
                            return;
                        };

                        let Some(PacketSpecificData::SupportedFunctions(s)) = supported
                            .into_iter()
                            .find(|v| matches!(v, PacketSpecificData::SupportedFunctions(_)))
                        else {
                            error!("got malformed message, disconnecting");
                            return;
                        };

                        let Some(data) = packet.payload() else {
                            error!("got malformed message, disconnecting");
                            return;
                        };

                        let Some((pid, data)) = crypto.new_connection(data) else {
                            error!("invalid login data");
                            return;
                        };

                        let data = create_packet_from(
                            LiteHeader {
                                destination_port: header.source_port,
                                source_port: param.virtual_port.get_port_number(),
                                stream_types: StreamTypes::new(
                                    param.virtual_port.get_stream_type(),
                                    header.stream_types.source(),
                                ),
                                fragment_id: 0,
                                sequence_id: 0,
                                types_flags: TypesFlags::default().types(CONNECT).flags(ACK),
                                ..Default::default()
                            },
                            &[
                                PacketSpecificData::SupportedFunctions(s & 0xFF),
                                PacketSpecificData::ConnectionSignature([0; 16]),
                            ],
                            &data,
                        );
                        websocket.send(Message::Binary(data.into())).await;

                        let addr = PRUDPSockAddr::new(
                            addr,
                            VirtualPort::new(header.source_port, header.stream_types.source()),
                        );
                        let Some(backend_conn) = new_backend_connection(&param, addr, pid).await
                        else {
                            error!("unable to connect to backend");
                            return;
                        };
                        let mut connection = ConnectionState {
                            active: true,
                            addr,
                            pid,
                            backend_conn,
                            client_reliable_counter: 2,
                            server_reliable_counter: 1,
                            param,
                            incoming_reliable: HashMap::new(),
                            websocket,
                        };

                        connection.handle_connection().await;
                        break;
                    }
                    v => {
                        error!(
                            "invalid packet type for unconnected client {}, disconnecting",
                            v,
                        );
                    }
                }
            }
            v => {
                error!("non binary message({:?}) , disconnecting", v);
                return;
            }
        }
    }
}

pub async fn start_proxy<C: Crypto>(param: ProxyStartupParam) {
    let param = Arc::new(param);
    let crypto = Arc::new(C::new());
    let listener = TcpListener::bind(param.self_private)
        .await
        .expect("unable to bind to port");

    while let Ok((connection, addr)) = listener.accept().await {
        let param = param.clone();
        let crypto = crypto.clone();
        tokio::spawn(websocket_thread_unconnected(
            param, crypto, connection, addr,
        ));
    }
}

pub async fn start_secure(param: ProxyStartupParam) {
    start_proxy::<Secure>(param).await;
}

pub async fn start_insecure(param: ProxyStartupParam) {
    start_proxy::<Insecure>(param).await;
}
