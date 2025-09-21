use std::{env, fs, io};
use std::hash::Hash;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddrV4, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use futures::{SinkExt, StreamExt};
use macros::{method_id, rmc_proto, rmc_struct, RmcSerialize};
use once_cell::sync::Lazy;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use rustls::client::WebPkiServerVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, TrustAnchor};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use webpki::anchor_from_trusted_cert;
use crate::common::setup;
use crate::define_rmc_proto;
use crate::nex::account::Account;
use crate::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote, RmcCallable, RmcConnection};
use rnex_core::rmc::response::ErrorCode;
use crate::rmc::structures::RmcSerialize;

pub trait UnitPacketRead: AsyncRead + Unpin{
    async fn read_buffer(&mut self) -> Result<Vec<u8>, io::Error>{
        let mut len_raw: [u8; 4] = [0; 4];

        self.read_exact(&mut len_raw).await?;

        let len = u32::from_le_bytes(len_raw);

        let mut vec = vec![0u8; len as _];

        self.read_exact(&mut vec).await?;

        Ok(vec)
    }
}

impl<T: AsyncRead + Unpin> UnitPacketRead for T{}
pub trait UnitPacketWrite: AsyncWrite + Unpin{
    async fn send_buffer(&mut self, data: &[u8]) -> Result<(), io::Error> {
        let mut dest_data = Vec::new();

        data.serialize(&mut dest_data).expect("ran out of memory or something");

        self.write_all(&dest_data[..]).await?;

        self.flush().await?;

        Ok(())
    }
}

impl<T: AsyncWrite + Unpin> UnitPacketWrite for T{}



#[rmc_proto(1)]
pub trait EdgeNodeManagement {
    #[method_id(1)]
    async fn get_url(&self, seed: u64) -> Result<SocketAddrV4, ErrorCode>;
}

define_rmc_proto!(
    proto EdgeNodeHolder{
        EdgeNodeManagement
    }
);

#[derive(RmcSerialize, Debug)]
#[repr(u32)]
pub enum EdgeNodeHolderConnectOption{
    DontRegister = 0,
    Register(SocketAddrV4) = 1
}
