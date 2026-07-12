#![allow(async_fn_in_trait)]
pub use tracing;
pub mod account;
pub mod date_time;
pub mod result;
pub mod station_url;

use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::vec;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Notify;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task;
use tracing::{error, info};

#[cfg(feature = "nx")]
pub type PID = i64;
#[cfg(not(feature = "nx"))]
pub type PID = i32;

pub trait UnitPacketRead: AsyncRead + Unpin {
    async fn read_buffer(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut len_raw: [u8; _] = [0; size_of::<usize>()];

        self.read_exact(&mut len_raw).await?;

        let len = usize::from_le_bytes(len_raw);

        let mut vec = vec![0u8; len as _];

        self.read_exact(&mut vec).await?;

        Ok(vec)
    }
}

impl<T: AsyncRead + Unpin> UnitPacketRead for T {}
pub trait UnitPacketWrite: AsyncWrite + Unpin {
    async fn send_buffer(&mut self, data: &[u8]) -> Result<(), io::Error> {
        let len_data = data.len().to_le_bytes();
        self.write_all(&len_data[..]).await?;
        self.write_all(data).await?;

        self.flush().await?;

        Ok(())
    }
}

impl<T: AsyncWrite + Unpin> UnitPacketWrite for T {}

#[derive(Clone, Debug)]
pub struct SendingBufferConnection(Sender<Vec<u8>>, Arc<Notify>);

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
    fn new<T: Send + Unpin + AsyncWrite + AsyncRead + 'static>(stream: T) -> Self {
        let (outside_send, inside_recv) = channel::<Vec<u8>>(10);
        let (inside_send, outside_recv) = channel::<Vec<u8>>(10);

        let notify = Arc::new(Notify::new());

        {
            let notify = notify.clone();
            task::spawn(async move {
                let sender = inside_send;
                let mut recver = inside_recv;
                let mut stream = stream;

                loop {
                    tokio::select! {
                        data = recver.recv() => {
                            let Some(data) = data else {
                                break;
                            };

                            if let Err(e) = stream.send_buffer(&data[..]).await{
                                error!("error sending data to backend: {e}");
                                break;
                            }
                        },
                        data = stream.read_buffer() => {
                            let data = match data{
                                Ok(d) => d,
                                Err(e) => {
                                    error!("error reveiving data from backend: {e}");
                                    break;
                                }
                            };

                            if let Err(e) = sender.send(data).await{
                                error!("a send error occurred {e}");
                                return;
                            }
                        },
                        () = notify.notified() => {
                            info!("shutting down connection");
                            break;
                        }
                    }
                }
                if let Err(e) = stream.shutdown().await {
                    error!("failed to shut down stream: {e}");
                }
            });
        }

        Self(SendingBufferConnection(outside_send, notify), outside_recv)
    }
}

impl SendingBufferConnection {
    pub async fn send(&self, buffer: Vec<u8>) -> Option<()> {
        self.0.send(buffer).await.ok()
    }
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.0.is_closed()
    }
    pub async fn disconnect(&self) {
        while !self.0.is_closed() {
            self.1.notify_waiters();
            tokio::task::yield_now().await;
        }
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
