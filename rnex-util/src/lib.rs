#![allow(async_fn_in_trait)]
pub use tracing;
pub mod account;
pub mod byte;
pub mod date_time;
pub mod diagnostics;
pub mod http;
#[cfg(feature = "nx")]
pub mod mystiko;
pub mod nnas;
pub mod result;
pub mod station_url;

use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::vec;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::sync::watch;
use tokio::task;
use tracing::{debug, error, info};

#[cfg(feature = "nx")]
pub type PID = i64;
#[cfg(not(feature = "nx"))]
pub type PID = i32;

pub const MAX_PACKET_SIZE: usize = 1024 * 1024;

pub trait UnitPacketRead: AsyncRead + Unpin {
    async fn read_buffer(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut len_raw = [0u8; 8];

        self.read_exact(&mut len_raw).await?;

        let len = usize::try_from(u64::from_le_bytes(len_raw))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "frame length overflow"))?;

        if len > MAX_PACKET_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "packet size {len} bytes exceeds maximum allowed limit of {MAX_PACKET_SIZE} bytes"
                ),
            ));
        }

        let mut vec = Vec::new();
        let mut limited = self.take(len as u64);
        limited.read_to_end(&mut vec).await?;
        if vec.len() != len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated frame",
            ));
        }

        Ok(vec)
    }
}

impl<T: AsyncRead + Unpin + ?Sized> UnitPacketRead for T {}

pub trait UnitPacketWrite: AsyncWrite + Unpin {
    async fn send_buffer(&mut self, data: &[u8]) -> Result<(), io::Error> {
        if data.len() > MAX_PACKET_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }
        let len_data = (data.len() as u64).to_le_bytes();
        self.write_all(&len_data[..]).await?;
        self.write_all(data).await?;
        self.flush().await?;

        Ok(())
    }
}

impl<T: AsyncWrite + Unpin + ?Sized> UnitPacketWrite for T {}

#[derive(Clone, Debug)]
pub struct SendingBufferConnection(Sender<Vec<u8>>, watch::Sender<bool>, u64);

static NEXT_DIAGNOSTIC_CONNECTION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct SplittableBufferConnection(SendingBufferConnection, Receiver<Vec<u8>>);

impl AsRef<SendingBufferConnection> for SplittableBufferConnection {
    fn as_ref(&self) -> &SendingBufferConnection {
        &self.0
    }
}

impl Deref for SplittableBufferConnection {
    type Target = SendingBufferConnection;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Send + Unpin + AsyncWrite + AsyncRead + 'static> From<T> for SplittableBufferConnection {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl SplittableBufferConnection {
    fn new<T>(stream: T) -> Self
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (outside_send, inside_recv) = channel::<Vec<u8>>(10);
        let (inside_send, outside_recv) = channel::<Vec<u8>>(10);

        let (notify, mut read_shutdown) = watch::channel(false);
        let mut write_shutdown = notify.subscribe();
        let (mut reader, mut writer) = io::split(stream);

        {
            let notify = notify.clone();
            task::spawn(async move {
                loop {
                    let result = tokio::select! {
                        _ = read_shutdown.changed() => break,
                        result = reader.read_buffer() => result,
                    };
                    match result {
                        Ok(data) => {
                            if !matches!(
                                tokio::time::timeout(
                                    std::time::Duration::from_secs(10),
                                    inside_send.send(data)
                                )
                                .await,
                                Ok(Ok(()))
                            ) {
                                // reciever dropped
                                break;
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                            debug!("backend connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("error receiving data from backend: {e:?}");
                            break;
                        }
                    }
                }
                notify.send_replace(true);
            });
        }

        {
            let notify = notify.clone();

            task::spawn(async move {
                let mut recver = inside_recv;

                loop {
                    tokio::select! {
                        data = recver.recv() => {
                            let Some(data) = data else {
                                break;
                            };

                            if let e @ (Err(_) | Ok(Err(_))) = tokio::time::timeout(std::time::Duration::from_secs(10), writer.send_buffer(&data[..])).await {
                                error!("error sending data to backend: {e:?}");
                                break;
                            }
                        },
                        _ = write_shutdown.changed() => {
                            info!("shutting down writer task");
                            break;
                        }
                    }
                }
                notify.send_replace(true);
                if let Err(e) = writer.shutdown().await {
                    error!("failed to shut down stream writer: {e}");
                }
            });
        }

        Self(
            SendingBufferConnection(
                outside_send,
                notify,
                NEXT_DIAGNOSTIC_CONNECTION.fetch_add(1, Ordering::Relaxed),
            ),
            outside_recv,
        )
    }
}

impl SendingBufferConnection {
    pub fn diagnostic_id(&self) -> u64 {
        self.2
    }

    pub async fn send(&self, buffer: Vec<u8>) -> Option<()> {
        self.0.send(buffer).await.ok()
    }
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.0.is_closed()
    }
    pub async fn disconnect(&self) {
        self.1.send_replace(true);
    }
}

impl SplittableBufferConnection {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.1.recv().await
    }

    #[must_use]
    pub fn duplicate_sender(&self) -> SendingBufferConnection {
        self.0.clone()
    }
}

pub struct WeakVec<T>(Vec<Weak<T>>);

impl<T> WeakVec<T> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn from_vec(vec: Vec<Weak<T>>) -> Self {
        Self(vec)
    }

    pub fn push(&mut self, val: Weak<T>) {
        self.0.retain(|v| v.upgrade().is_some());

        self.0.push(val);
    }

    pub fn iter(&self) -> impl Iterator<Item = Arc<T>> {
        self.0.iter().filter_map(Weak::upgrade)
    }
}
