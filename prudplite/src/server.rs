use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use proxy_common::RNEX_ACCESS_KEY;
use rnex_server::ConnectionInitData;
use rnex_server::rmc::serialization::RmcSerialize;
use rnex_util::{UnitPacketRead, UnitPacketWrite};
use rnex_util::diagnostics;
use sha1::{Digest, Sha1};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, interval, interval_at, sleep_until, timeout};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tracing::{debug, error, info, warn};

use crate::packet::{
    ACK, CONNECT, DATA, DISCONNECT, HAS_SIZE, MULTI_ACK, NEED_ACK, PING, Packet, RELIABLE, SYN,
    decode_packets,
};
use rnex_prudp::lite::{connect_signature, connection_signature, derive_key, verify_login};

const MAX_WS_MESSAGE: usize = 1024 * 1024;
static NEXT_AUTH_CONNECTION: AtomicI64 = AtomicI64::new(-1);

#[derive(Clone, Copy)]
struct Backends {
    auth: SocketAddr,
    secure: SocketAddr,
}

fn required_env(name: &str) -> io::Result<String> {
    std::env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} is required")))
}

fn configured_address(name: &str) -> io::Result<SocketAddr> {
    required_env(name)?.parse().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {name}: {err}"),
        )
    })
}

pub async fn start() -> io::Result<()> {
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    let bind = std::env::var("RNEX_LITE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:443".to_owned())
        .parse::<SocketAddr>()
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid RNEX_LITE_BIND: {err}"),
            )
        })?;
    let backends = Backends {
        auth: configured_address("RNEX_LITE_AUTH_BACKEND")?,
        secure: configured_address("RNEX_LITE_SECURE_BACKEND")?,
    };
    let password = required_env("RNEX_SERVER_PASSWORD")?;
    let secure_key = derive_key(&password, 2);
    let cert_path = required_env("RNEX_LITE_TLS_CERT")?;
    let key_path = required_env("RNEX_LITE_TLS_KEY")?;
    let certs = CertificateDer::pem_file_iter(cert_path)
        .map_err(io::Error::other)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    let key = PrivateKeyDer::from_pem_file(key_path).map_err(io::Error::other)?;
    let tls = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(io::Error::other)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    let listener = TcpListener::bind(bind).await?;
    info!(address = %bind, "PRUDP Lite listening");
    diagnostics::record("lite_listening", format_args!("address={bind}"));
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                error!(%err, "PRUDP Lite accept failed");
                continue;
            }
        };
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let result = async {
                let tls = timeout(Duration::from_secs(5), acceptor.accept(stream))
                    .await
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::TimedOut, "TLS handshake timed out")
                    })??;
                run_connection(tls, peer, backends, secure_key).await
            }
            .await;
            if let Err(err) = result {
                debug!(%peer, %err, "PRUDP Lite connection closed");
                diagnostics::record("lite_closed", format_args!("peer={peer} error={err:?}"));
            }
        });
    }
}

async fn upgrade(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    peer: SocketAddr,
) -> io::Result<SocketAddr> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 8192 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "WebSocket header too large",
            ));
        }
        let mut byte = [0];
        stream.read_exact(&mut byte).await?;
        header.push(byte[0]);
    }
    let text = std::str::from_utf8(&header)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP header"))?;
    let mut lines = text.split("\r\n");
    let request = lines.next().unwrap_or_default();
    if !request.starts_with("GET ") || !request.ends_with(" HTTP/1.1") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WebSocket request",
        ));
    }
    let mut key = None;
    let mut upgrade = false;
    let mut connection = false;
    let mut nex = false;
    let mut forwarded_ip = None;
    let mut forwarded_port = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("Sec-WebSocket-Key") {
            key = Some(value);
        } else if name.eq_ignore_ascii_case("Upgrade") {
            upgrade = value.eq_ignore_ascii_case("websocket");
        } else if name.eq_ignore_ascii_case("Connection") {
            connection = value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case("upgrade"));
        } else if name.eq_ignore_ascii_case("Sec-WebSocket-Protocol") {
            nex = value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case("NEX"));
        } else if name.eq_ignore_ascii_case("X-Forwarded-For") {
            forwarded_ip = value
                .split(',')
                .next()
                .and_then(|ip| ip.trim().parse().ok());
        } else if name.eq_ignore_ascii_case("X-Real-IP") && forwarded_ip.is_none() {
            forwarded_ip = value.parse().ok();
        } else if name.eq_ignore_ascii_case("X-Forwarded-Port") {
            forwarded_port = value.parse().ok();
        }
    }
    let Some(key) = key else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing WebSocket key",
        ));
    };
    if !upgrade || !connection || key.len() > 256 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WebSocket upgrade",
        ));
    }
    let mut digest = Sha1::new();
    digest.update(key.as_bytes());
    digest.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let accept = STANDARD.encode(digest.finalize());
    let protocol = if nex {
        "Sec-WebSocket-Protocol: NEX\r\n"
    } else {
        ""
    };
    let answer = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n{protocol}\r\n"
    );
    stream.write_all(answer.as_bytes()).await?;
    if std::env::var("RNEX_LITE_TRUST_PROXY_HEADERS").as_deref() == Ok("1") {
        if let Some(ip) = forwarded_ip {
            return Ok(SocketAddr::new(ip, forwarded_port.unwrap_or(peer.port())));
        }
    }
    Ok(peer)
}

async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<(u8, bool, Vec<u8>)> {
    let mut head = [0u8; 2];
    reader.read_exact(&mut head).await?;
    if head[0] & 0x70 != 0 || head[1] & 0x80 == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WebSocket frame",
        ));
    }
    let opcode = head[0] & 0x0f;
    let fin = head[0] & 0x80 != 0;
    let mut size = (head[1] & 0x7f) as u64;
    if size == 126 {
        let mut bytes = [0; 2];
        reader.read_exact(&mut bytes).await?;
        size = u16::from_be_bytes(bytes) as u64;
    } else if size == 127 {
        let mut bytes = [0; 8];
        reader.read_exact(&mut bytes).await?;
        size = u64::from_be_bytes(bytes);
    }
    if size > MAX_WS_MESSAGE as u64 || opcode >= 8 && (!fin || size > 125) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WebSocket frame too large",
        ));
    }
    let mut mask = [0; 4];
    reader.read_exact(&mut mask).await?;
    let mut data = vec![0; size as usize];
    reader.read_exact(&mut data).await?;
    for (index, byte) in data.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    Ok((opcode, fin, data))
}

async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    opcode: u8,
    data: &[u8],
) -> io::Result<()> {
    let mut header = vec![0x80 | opcode];
    if data.len() < 126 {
        header.push(data.len() as u8);
    } else if data.len() <= u16::MAX as usize {
        header.push(126);
        header.extend_from_slice(&(data.len() as u16).to_be_bytes());
    } else {
        header.push(127);
        header.extend_from_slice(&(data.len() as u64).to_be_bytes());
    }
    writer.write_all(&header).await?;
    writer.write_all(data).await?;
    writer.flush().await
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct SessionKey {
    source_type: u8,
    source_port: u8,
    destination_type: u8,
    destination_port: u8,
}

impl From<&Packet> for SessionKey {
    fn from(packet: &Packet) -> Self {
        Self {
            source_type: packet.source_type,
            source_port: packet.source_port,
            destination_type: packet.destination_type,
            destination_port: packet.destination_port,
        }
    }
}

enum HandleOutcome {
    None,
    Request(Vec<u8>),
    Disconnect,
}

fn is_nat_initiate_probe(data: &[u8]) -> bool {
    data.len() >= 13 && data[4] == 0x83 && u32::from_le_bytes(data[9..13].try_into().unwrap()) == 2
}

struct SessionEntry {
    session: Session,
    backend: Option<OwnedWriteHalf>,
    backend_reader: Option<JoinHandle<()>>,
    backend_closed: bool,
    created: Instant,
}

impl SessionEntry {
    fn new(peer: SocketAddr, secure_key: Option<[u8; 16]>) -> Self {
        Self {
            session: Session::new(peer, secure_key),
            backend: None,
            backend_reader: None,
            backend_closed: false,
            created: Instant::now(),
        }
    }

    async fn connect_backend(
        &mut self,
        key: SessionKey,
        destination: SocketAddr,
        peer: SocketAddr,
        responses: mpsc::Sender<(SessionKey, io::Result<Vec<u8>>)>,
    ) -> io::Result<()> {
        let mut stream = TcpStream::connect(destination).await?;
        let metadata = ConnectionInitData {
            addr: peer,
            pid: self.session.pid.unwrap(),
        }
        .to_data()
        .map_err(io::Error::other)?;
        diagnostics::record_bytes("backend_metadata_out", format_args!("peer={peer} port={}", key.destination_port), &metadata);
        stream.send_buffer(&metadata).await?;
        let (mut reader, writer) = stream.into_split();
        self.backend = Some(writer);
        self.backend_reader = Some(tokio::spawn(async move {
            loop {
                let result = reader.read_buffer().await;
                let ended = result.is_err();
                if responses.send((key, result)).await.is_err() || ended {
                    break;
                }
            }
        }));
        Ok(())
    }
}

impl Drop for SessionEntry {
    fn drop(&mut self) {
        if let Some(reader) = self.backend_reader.take() {
            reader.abort();
        }
    }
}

struct Session {
    peer: SocketAddr,
    secure_key: Option<[u8; 16]>,
    signature: [u8; 16],
    source_type: u8,
    source_port: u8,
    destination_type: u8,
    destination_port: u8,
    minor_version: u8,
    supported_functions: u32,
    syn_seen: bool,
    pid: Option<rnex_util::PID>,
    connect_sequence: Option<u16>,
    connect_response: Option<Packet>,
    next_sequence: u16,
    received: VecDeque<u16>,
    pending: BTreeMap<u16, Vec<u8>>,
    nat_probe_sequences: HashSet<u16>,
    outgoing: VecDeque<Packet>,
    fragments: Vec<u8>,
}

impl Session {
    fn new(peer: SocketAddr, secure_key: Option<[u8; 16]>) -> Self {
        Self {
            peer,
            secure_key,
            signature: connection_signature(peer),
            source_type: 0,
            source_port: 0,
            destination_type: 0,
            destination_port: 0,
            minor_version: 5,
            supported_functions: std::env::var("RNEX_LITE_SUPPORTED_FUNCTIONS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            syn_seen: false,
            pid: None,
            connect_sequence: None,
            connect_response: None,
            next_sequence: 1,
            received: VecDeque::with_capacity(128),
            pending: BTreeMap::new(),
            nat_probe_sequences: HashSet::new(),
            outgoing: VecDeque::new(),
            fragments: Vec::new(),
        }
    }

    fn reply(&self, packet: &Packet, flags: u16, payload: Vec<u8>) -> Packet {
        Packet {
            kind: packet.kind,
            flags,
            source_type: self.source_type,
            source_port: self.source_port,
            destination_type: self.destination_type,
            destination_port: self.destination_port,
            fragment_id: packet.fragment_id,
            sequence_id: packet.sequence_id,
            minor_version: self.minor_version,
            supported_functions: self.supported_functions,
            payload,
            ..Default::default()
        }
    }

    async fn send_packet(
        &mut self,
        writer: &mut (impl AsyncWrite + Unpin),
        packet: Packet,
        reliable: bool,
    ) -> io::Result<()> {
        let encoded = packet.encode().map_err(io::Error::other)?;
        diagnostics::record_bytes("lite_packet_out", format_args!("peer={} packet={packet:?} reliable={reliable}", self.peer), &encoded);
        if reliable && self.pending.len() >= 512 {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "too many unacknowledged Lite packets",
            ));
        }
        write_frame(writer, 2, &encoded).await?;
        if reliable {
            self.pending.insert(packet.sequence_id, encoded);
        }
        Ok(())
    }

    async fn handle(
        &mut self,
        packet: Packet,
        writer: &mut (impl AsyncWrite + Unpin),
    ) -> io::Result<HandleOutcome> {
        if packet.kind == SYN && packet.flags & ACK == 0 && self.pid.is_none() {
            self.source_type = packet.destination_type;
            self.source_port = packet.destination_port;
            self.destination_type = packet.source_type;
            self.destination_port = packet.source_port;
            self.minor_version = self.minor_version.min(packet.minor_version);
            self.supported_functions &= packet.supported_functions;
            self.received.clear();
            self.fragments.clear();
            self.syn_seen = true;
            let mut answer = self.reply(&packet, ACK | HAS_SIZE, Vec::new());
            answer.connection_signature = self.signature.to_vec();
            self.send_packet(writer, answer, false).await?;
            return Ok(HandleOutcome::None);
        }
        if packet.kind == CONNECT && packet.flags & ACK == 0 && self.pid.is_some() {
            if self.connect_sequence == Some(packet.sequence_id)
                && packet.signature.len() == 16
                && connect_signature(RNEX_ACCESS_KEY, &self.signature)
                    .ct_eq(&packet.signature)
                    .unwrap_u8()
                    == 1
            {
                if let Some(response) = self.connect_response.clone() {
                    self.send_packet(writer, response, false).await?;
                }
            }
            return Ok(HandleOutcome::None);
        }
        if packet.kind == CONNECT && packet.flags & ACK == 0 && self.pid.is_none() {
            if !self.syn_seen
                || packet.flags & NEED_ACK == 0
                || packet.signature.len() != 16
                || connect_signature(RNEX_ACCESS_KEY, &self.signature)
                    .ct_eq(&packet.signature)
                    .unwrap_u8()
                    != 1
            {
                warn!(peer = %self.peer, "PRUDP Lite CONNECT signature rejected");
                return Ok(HandleOutcome::None);
            }
            let (pid, answer) = match self.secure_key {
                Some(key) => {
                    let Some((pid, answer)) = verify_login(&packet.payload, &key) else {
                        warn!(peer = %self.peer, "PRUDP Lite secure ticket rejected");
                        return Ok(HandleOutcome::None);
                    };
                    let Ok(pid) = rnex_util::PID::try_from(pid) else {
                        return Ok(HandleOutcome::None);
                    };
                    (pid, answer)
                }
                None if packet.payload.is_empty() => (
                    NEXT_AUTH_CONNECTION.fetch_sub(1, Ordering::Relaxed) as rnex_util::PID,
                    Vec::new(),
                ),
                None => return Ok(HandleOutcome::None),
            };
            let mut response = self.reply(&packet, ACK | HAS_SIZE, answer);
            response.sequence_id = 1;
            self.connect_sequence = Some(packet.sequence_id);
            self.connect_response = Some(response.clone());
            self.send_packet(writer, response, false).await?;
            self.pid = Some(pid);
            info!(peer = %self.peer, pid, secure = self.secure_key.is_some(), "PRUDP Lite connected");
            return Ok(HandleOutcome::None);
        }
        if self.pid.is_none() {
            return Ok(HandleOutcome::None);
        }
        if packet.kind == DATA && packet.flags & MULTI_ACK != 0 {
            if packet.payload.len() >= 4 {
                let count = packet.payload[1] as usize;
                let base = u16::from_le_bytes([packet.payload[2], packet.payload[3]]);
                self.pending.retain(|id, _| *id > base);
                let acknowledged: Vec<_> = self
                    .nat_probe_sequences
                    .iter()
                    .copied()
                    .filter(|id| *id <= base)
                    .collect();
                for id in acknowledged {
                    self.nat_probe_sequences.remove(&id);
                    info!(peer = %self.peer, sequence_id = id, "NAT InitiateProbe acknowledged by client");
                }
                for entry in packet.payload[4..].chunks_exact(2).take(count) {
                    let id = u16::from_le_bytes([entry[0], entry[1]]);
                    self.pending.remove(&id);
                    if self.nat_probe_sequences.remove(&id) {
                        info!(peer = %self.peer, sequence_id = id, "NAT InitiateProbe acknowledged by client");
                    }
                }
            }
            return Ok(HandleOutcome::None);
        }
        if packet.flags & ACK != 0 {
            if packet.kind == DATA {
                self.pending.remove(&packet.sequence_id);
                if self.nat_probe_sequences.remove(&packet.sequence_id) {
                    info!(peer = %self.peer, sequence_id = packet.sequence_id, "NAT InitiateProbe acknowledged by client");
                }
            }
            return Ok(HandleOutcome::None);
        }
        if packet.kind == DISCONNECT {
            if packet.flags & NEED_ACK != 0 {
                let response = self.reply(&packet, ACK, Vec::new());
                for _ in 0..3 {
                    self.send_packet(writer, response.clone(), false).await?;
                }
            }
            return Ok(HandleOutcome::Disconnect);
        }
        if packet.flags & NEED_ACK != 0 {
            let response = self.reply(&packet, ACK, Vec::new());
            self.send_packet(writer, response, false).await?;
        }
        match packet.kind {
            DATA => {
                if packet.flags & NEED_ACK != 0 {
                    if self.received.contains(&packet.sequence_id) {
                        return Ok(HandleOutcome::None);
                    }
                    if self.received.len() == 128 {
                        self.received.pop_front();
                    }
                    self.received.push_back(packet.sequence_id);
                }
                if self.fragments.len() + packet.payload.len() > MAX_WS_MESSAGE {
                    self.fragments.clear();
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "fragmented NEX request too large",
                    ));
                }
                self.fragments.extend_from_slice(&packet.payload);
                if packet.fragment_id == 0 {
                    return Ok(HandleOutcome::Request(std::mem::take(&mut self.fragments)));
                }
            }
            PING => {}
            _ => debug!(kind = packet.kind, "unhandled PRUDP Lite packet type"),
        }
        Ok(HandleOutcome::None)
    }

    fn queue_data(&mut self, data: &[u8]) -> io::Result<()> {
        if data.is_empty() || data == [0, 0, 0, 0, 0] {
            return Ok(());
        }
        if self.outgoing.len() + data.len().div_ceil(1300) > 1024 {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "too many queued Lite packets",
            ));
        }
        for (index, chunk) in data.chunks(1300).enumerate() {
            let remaining = data.len().saturating_sub((index + 1) * 1300);
            let fragment_id = if remaining == 0 {
                0
            } else {
                ((index % 255) + 1) as u8
            };
            let packet = Packet {
                kind: DATA,
                flags: RELIABLE | NEED_ACK,
                source_type: self.source_type,
                source_port: self.source_port,
                destination_type: self.destination_type,
                destination_port: self.destination_port,
                fragment_id,
                sequence_id: self.next_sequence,
                payload: chunk.to_vec(),
                ..Default::default()
            };
            self.next_sequence = self.next_sequence.wrapping_add(1);
            self.outgoing.push_back(packet);
        }
        Ok(())
    }
}

async fn run_connection(
    mut socket: impl AsyncRead + AsyncWrite + Unpin,
    peer: SocketAddr,
    backends: Backends,
    secure_key: [u8; 16],
) -> io::Result<()> {
    let client = timeout(Duration::from_secs(5), upgrade(&mut socket, peer))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "WebSocket handshake timed out"))??;
    diagnostics::record("lite_connected", format_args!("peer={peer} client={client}"));
    let (mut reader, mut writer) = tokio::io::split(socket);
    let mut sessions: HashMap<SessionKey, SessionEntry> = HashMap::new();
    let (responses, mut incoming) = mpsc::channel(64);
    let mut bytes = Vec::new();
    let mut retransmit = interval_at(
        Instant::now() + Duration::from_secs(2),
        Duration::from_secs(2),
    );
    let mut fragment_clock = interval(Duration::from_millis(15));
    let mut session_clock = interval(Duration::from_secs(1));
    let mut last_seen = Instant::now();
    let mut fragmented_ws = Vec::new();
    let mut ws_fragment_active = false;
    loop {
        tokio::select! {
            _ = sleep_until(last_seen + Duration::from_secs(600)) => {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "PRUDP Lite connection idle"));
            }
            frame = read_frame(&mut reader) => {
                let (opcode, fin, data) = frame?;
                diagnostics::record_bytes("websocket_in", format_args!("peer={client} opcode={opcode} fin={fin}"), &data);
                last_seen = Instant::now();
                match opcode {
                    8 => {
                        write_frame(&mut writer, 8, &data).await?;
                        return Ok(());
                    }
                    9 => {
                        write_frame(&mut writer, 10, &data).await?;
                        continue;
                    }
                    10 => continue,
                    2 if !ws_fragment_active => {
                        fragmented_ws.extend_from_slice(&data);
                        ws_fragment_active = !fin;
                    }
                    0 if ws_fragment_active => {
                        fragmented_ws.extend_from_slice(&data);
                        ws_fragment_active = !fin;
                    }
                    _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected WebSocket opcode")),
                }
                if fragmented_ws.len() > MAX_WS_MESSAGE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "WebSocket message too large"));
                }
                if ws_fragment_active { continue; }
                bytes.extend_from_slice(&fragmented_ws);
                fragmented_ws.clear();
                if bytes.len() > MAX_WS_MESSAGE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "PRUDP Lite stream too large"));
                }
                let (packets, consumed) = decode_packets(&bytes).map_err(io::Error::other)?;
                bytes.drain(..consumed);
                for packet in packets {
                    diagnostics::record("lite_packet_in", format_args!("peer={client} packet={packet:?}"));
                    let key = SessionKey::from(&packet);
                    let destination = match key.destination_port {
                        1 => backends.auth,
                        2 => backends.secure,
                        _ => {
                            debug!(port = key.destination_port, "unknown PRUDP Lite server port");
                            continue;
                        }
                    };
                    if packet.kind == SYN && packet.flags & ACK == 0 {
                        if sessions.get(&key).is_some_and(|entry| entry.session.pid.is_some()) {
                            sessions.remove(&key);
                        }
                        if !sessions.contains_key(&key) {
                            if sessions.len() >= 64 {
                                return Err(io::Error::new(io::ErrorKind::OutOfMemory, "too many Lite sessions"));
                            }
                            let key_material = (key.destination_port == 2).then_some(secure_key);
                            sessions.insert(key, SessionEntry::new(client, key_material));
                        }
                    }
                    let Some(entry) = sessions.get_mut(&key) else {
                        continue;
                    };
                    let outcome = entry.session.handle(packet, &mut writer).await?;
                    match outcome {
                        HandleOutcome::Request(request) => {
                            diagnostics::record_bytes("backend_rmc_out", format_args!("peer={client} port={}", key.destination_port), &request);
                            if let Some(backend) = entry.backend.as_mut() {
                                info!(
                                    peer = %client,
                                    port = key.destination_port,
                                    request_bytes = request.len(),
                                    "PRUDP Lite forwarding RMC request"
                                );
                                backend.send_buffer(&request).await?;
                            } else {
                                warn!(
                                    peer = %client,
                                    port = key.destination_port,
                                    request_bytes = request.len(),
                                    "PRUDP Lite RMC request has no backend"
                                );
                            }
                        }
                        HandleOutcome::Disconnect => {
                            sessions.remove(&key);
                            continue;
                        }
                        HandleOutcome::None => {}
                    }
                    let Some(entry) = sessions.get_mut(&key) else {
                        continue;
                    };
                    if entry.backend.is_none() && !entry.backend_closed && entry.session.pid.is_some() {
                        if let Err(err) = entry.connect_backend(key, destination, client, responses.clone()).await {
                            warn!(%client, %err, port = key.destination_port, "Lite backend connection failed");
                            sessions.remove(&key);
                        } else {
                            info!(
                                peer = %client,
                                port = key.destination_port,
                                "PRUDP Lite backend connected"
                            );
                        }
                    }
                }
            }
            Some((key, response)) = incoming.recv() => {
                let Some(entry) = sessions.get_mut(&key) else {
                    continue;
                };
                match response {
                    Ok(data) => {
                        diagnostics::record_bytes("backend_rmc_in", format_args!("peer={client} port={}", key.destination_port), &data);
                        if is_nat_initiate_probe(&data) {
                            info!(peer = %client, port = key.destination_port, response_bytes = data.len(), "PRUDP Lite received NAT InitiateProbe from backend");
                        }
                        info!(
                            peer = %client,
                            port = key.destination_port,
                            response_bytes = data.len(),
                            "PRUDP Lite forwarding RMC response"
                        );
                        entry.session.queue_data(&data)?;
                    }
                    Err(err) => {
                        diagnostics::record("backend_closed", format_args!("peer={client} port={} error={err:?}", key.destination_port));
                        debug!(%client, %err, port = key.destination_port, "Lite backend closed");
                        entry.backend = None;
                        entry.backend_closed = true;
                        if entry.session.outgoing.is_empty() {
                            sessions.remove(&key);
                        }
                    }
                }
            }
            _ = fragment_clock.tick(), if sessions.values().any(|entry| !entry.session.outgoing.is_empty()) => {
                let mut closed = Vec::new();
                for (key, entry) in &mut sessions {
                    if let Some(packet) = entry.session.outgoing.pop_front() {
                        let probe = is_nat_initiate_probe(&packet.payload);
                        let sequence_id = packet.sequence_id;
                        entry.session.send_packet(&mut writer, packet, true).await?;
                        if probe {
                            entry.session.nat_probe_sequences.insert(sequence_id);
                            info!(peer = %client, port = key.destination_port, sequence_id, "PRUDP Lite sent NAT InitiateProbe to client");
                        }
                    }
                    if entry.backend_closed && entry.session.outgoing.is_empty() {
                        closed.push(*key);
                    }
                }
                for key in closed {
                    sessions.remove(&key);
                }
            }
            _ = retransmit.tick() => {
                for entry in sessions.values() {
                    for data in entry.session.pending.values() {
                        diagnostics::record_bytes("lite_packet_retransmit", format_args!("peer={client}"), data);
                        write_frame(&mut writer, 2, data).await?;
                    }
                }
            }
            _ = session_clock.tick() => {
                sessions.retain(|_, entry| entry.session.pid.is_some() || entry.created.elapsed() < Duration::from_secs(10));
            }
        }
    }
}