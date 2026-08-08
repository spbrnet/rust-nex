#![allow(async_fn_in_trait)]

pub mod helpers;
pub mod message;
pub mod primitives;
pub mod qresult;
pub mod response;
pub mod rmc_struct;
pub mod station_url;
use std::{collections::HashMap, fmt::Debug, io::Cursor, ops::Deref, sync::Arc, time::Duration};

pub use rand;
pub use rnex_rmc_macros::*;
use rnex_util::{SendingBufferConnection, SplittableBufferConnection, result::ResultExtension};
use tokio::{
    sync::{Mutex, Notify},
    task,
    time::{Instant, sleep, sleep_until},
};
pub use tracing;
pub mod any;
pub mod buffer;
pub mod data;
pub mod date_time;
pub mod list;
pub mod networking;
pub mod qbuffer;
pub mod serialization;
pub mod string;
pub mod string_set;
pub mod variant;

use thiserror::Error;

pub mod config {
    pub const FEATURE_HAS_STRUCT_HEADER: bool = cfg!(feature = "rmc_struct_header");
}

pub use paste;
pub use rnex_util as util;
use tracing::{error, info};

use crate::{
    RemoteCallError::ConnectionBroke,
    message::RMCMessage,
    response::{ErrorCode, RMCResponse, RMCResponseResult},
    serialization::RmcSerialize,
};

#[derive(Error, Debug)]
pub enum RemoteCallError {
    #[error("Call to remote timed out whilst waiting on response.")]
    Timeout,
    #[error("A server side rmc error occurred: {0:?}")]
    ServerError(ErrorCode),
    #[error("Connection broke")]
    ConnectionBroke,
    #[error("Error reading response data: {0}")]
    InvalidResponse(#[from] serialization::Error),
}

#[derive(Clone, Debug)]
pub struct RmcConnection(pub SendingBufferConnection, pub RmcResponseReceiver);

#[derive(Clone, Debug)]
pub struct RmcResponseReceiver(Arc<Notify>, Arc<Mutex<HashMap<u32, RMCResponse>>>);

impl RmcConnection {
    pub async fn make_raw_call<T: RmcSerialize>(
        &self,
        message: &RMCMessage,
    ) -> Result<T, RemoteCallError> {
        self.make_raw_call_no_response(message).await?;

        let data = self.1.get_response_data(message.call_id).await?;

        let out = <T as RmcSerialize>::deserialize(&mut Cursor::new(data))?;

        Ok(out)
    }

    pub async fn make_raw_call_no_response(
        &self,
        message: &RMCMessage,
    ) -> Result<(), RemoteCallError> {
        let message_data = message.to_data();

        self.0.send(message_data).await.ok_or(ConnectionBroke)?;

        Ok(())
    }

    pub async fn disconnect(&self) {
        self.0.disconnect().await;
    }
}

pub trait HasProtoId<const ID: u16> {}

impl RmcResponseReceiver {
    // returns none if timed out
    pub async fn get_response_data(&self, call_id: u32) -> Result<Vec<u8>, RemoteCallError> {
        let mut end_wait_time = Instant::now();
        end_wait_time += Duration::from_secs(5);

        let sleep_fut = sleep_until(end_wait_time);
        tokio::pin!(sleep_fut);

        let mut sleep_manual_unlock_fut = Instant::now();
        sleep_manual_unlock_fut += Duration::from_secs(4);

        let sleep_manual_unlock_fut = sleep_until(sleep_manual_unlock_fut);
        tokio::pin!(sleep_manual_unlock_fut);

        loop {
            let mut locked = self.1.lock().await;

            if let Some(v) = locked.remove(&call_id) {
                match v.response_result {
                    RMCResponseResult::Success { data, .. } => return Ok(data),
                    RMCResponseResult::Error { error_code, .. } => {
                        return Err(RemoteCallError::ServerError(error_code));
                    }
                }
            }

            drop(locked);

            let notif_fut = self.0.notified();

            tokio::select! {
                _ = &mut sleep_manual_unlock_fut => {
                    continue;
                }
                _ = &mut sleep_fut => {
                    return Err(RemoteCallError::Timeout);
                }
                _ = notif_fut => {
                    continue;
                }
            }
        }
    }
}

pub trait HasRmcConnection {
    fn get_connection(&self) -> &RmcConnection;
}

pub trait RemoteObject {
    fn new(conn: RmcConnection) -> Self;
}

impl RemoteObject for () {
    fn new(_: RmcConnection) -> Self {}
}

pub trait RmcCallable {
    //type Remote: RemoteObject;
    // returns false on fail to match protocol to an implementation
    fn rmc_call(
        &self,
        responder: &SendingBufferConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: &[u8],
    ) -> impl std::future::Future<Output = bool> + Send;
}

impl<T: RmcCallable + Sync + Send> RmcCallable for Arc<T> {
    async fn rmc_call(
        &self,
        responder: &SendingBufferConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: &[u8],
    ) -> bool {
        self.as_ref()
            .rmc_call(responder, protocol_id, method_id, call_id, rest)
            .await
    }
}

impl<T: RmcCallable + Sync + Send> RmcCallable for Option<T> {
    async fn rmc_call(
        &self,
        responder: &SendingBufferConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: &[u8],
    ) -> bool {
        if let Some(callable) = self.as_ref() {
            return callable
                .rmc_call(responder, protocol_id, method_id, call_id, rest)
                .await;
        }
        false
    }
}

impl<T: RmcCallable + Sync + Send> RmcCallable for Box<T> {
    async fn rmc_call(
        &self,
        responder: &SendingBufferConnection,
        protocol_id: u16,
        method_id: u32,
        call_id: u32,
        rest: &[u8],
    ) -> bool {
        self.as_ref()
            .rmc_call(responder, protocol_id, method_id, call_id, rest)
            .await
    }
}

#[macro_export]
macro_rules! define_rmc_proto {
    (proto $name:ident{
        $($protocol:path),* $(,)?
    }) => {
        $crate::paste::paste!{
            #[allow(unused_variables)]
            pub trait [<Local $name>]: std::any::Any $( + [<Raw $protocol>] + $protocol)* {
                async fn rmc_call(&self, remote_response_connection: &$crate::util::SendingBufferConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: &[u8]) -> bool{
                    match protocol_id{
                        $(
                            [<Raw $protocol Info>]::PROTOCOL_ID => {<Self as [<Raw $protocol>]>::rmc_call_proto(self, remote_response_connection, method_id, call_id, rest).await; true},
                        )*
                        v => false
                    }
                }
            }

            #[derive(Debug)]
            pub struct [<Remote $name>]($crate::RmcConnection);

            impl $crate::RmcPureRemoteObject for [<Remote $name>]{
                fn new(conn: $crate::RmcConnection) -> Self{
                    Self(conn)
                }
            }

            impl $crate::RemoteDisconnectable for [<Remote $name>]{

                async fn disconnect(&self){
                    self.0.disconnect().await;
                }
            }

            impl $crate::HasRmcConnection for [<Remote $name>]{
                fn get_connection(&self) -> &$crate::RmcConnection{
                    &self.0
                }
            }

            $(
            impl [<Remote $protocol>] for [<Remote $name>]{}
            )*
        }
    };
}

/// This is a special case to allow unit to represent the fact that no object is represented.
impl RmcCallable for () {
    async fn rmc_call(
        &self,
        _remote_response_connection: &SendingBufferConnection,
        _protocol_id: u16,
        _method_id: u32,
        _call_id: u32,
        _rest: &[u8],
    ) -> bool {
        false
    }
}

pub trait RmcPureRemoteObject {
    fn new(conn: RmcConnection) -> Self;
}

pub trait RemoteDisconnectable {
    async fn disconnect(&self);
}

#[derive(Debug)]
pub struct OnlyRemote<T: RemoteDisconnectable>(T);

impl<T: RemoteDisconnectable + RmcPureRemoteObject> OnlyRemote<T> {
    pub fn new(conn: RmcConnection) -> Self {
        Self(T::new(conn))
    }
}

impl<T: RemoteDisconnectable> Deref for OnlyRemote<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: RemoteDisconnectable> OnlyRemote<T> {
    pub async fn disconnect(&self) {
        self.0.disconnect().await;
    }
}

impl<T: RemoteDisconnectable> RmcCallable for OnlyRemote<T> {
    fn rmc_call(
        &self,
        _responder: &SendingBufferConnection,
        _protocol_id: u16,
        _method_id: u32,
        _call_id: u32,
        _rest: &[u8],
    ) -> impl Future<Output = bool> + Send {
        // maybe respond with not implemented or something
        async { false }
    }
}

async fn handle_incoming<T: RmcCallable + Send + Sync + Debug + 'static>(
    sending_conn: SendingBufferConnection,
    remote: Arc<T>,
    notify: Arc<Notify>,
    incoming: Arc<Mutex<HashMap<u32, RMCResponse>>>,
    data: Vec<u8>,
) {
    let Some(proto_id) = data.get(4) else {
        error!("received too small rmc message.");
        error!("ending rmc gateway.");
        sending_conn.disconnect().await;
        return;
    };

    // protocol 0 is hardcoded to be the no protocol protocol aka keepalive protocol
    if *proto_id == 0 {
        println!("got keepalive");
        return;
    }

    if (proto_id & 0x80) == 0 {
        let Some(response) = RMCResponse::new(&mut Cursor::new(data)).display_err_or_some() else {
            error!("invalid rmc response.");
            error!("ending rmc gateway.");
            sending_conn.disconnect().await;
            return;
        };

        info!("got rmc response");

        let mut locked = incoming.lock().await;

        locked.insert(response.get_call_id(), response);
        notify.notify_waiters();
    } else {
        let Some(message) = RMCMessage::new(&mut Cursor::new(data)).display_err_or_some() else {
            error!("invalid rmc message.");
            error!("ending rmc gateway.");
            sending_conn.disconnect().await;
            return;
        };

        let RMCMessage {
            protocol_id,
            method_id,
            call_id,
            rest_of_data,
        } = message;

        async {
            if !remote
                .rmc_call(
                    &sending_conn,
                    protocol_id,
                    method_id,
                    call_id,
                    &rest_of_data[..],
                )
                .await
            {
                error!(
                    protocol_id,
                    method_id,
                    arguments = hex::encode(&rest_of_data),
                    "rmc call on unimplemented protocol"
                )
            }
        }
        .await;
    }
}

async fn handle_incoming_loop<T: RmcCallable + Send + Sync + Debug + 'static>(
    mut connection: SplittableBufferConnection,
    remote: Arc<T>,
    notify: Arc<Notify>,
    incoming: Arc<Mutex<HashMap<u32, RMCResponse>>>,
) {
    while let Some(data) = connection.recv().await {
        let sending_conn = connection.duplicate_sender();
        let remote = remote.clone();
        let notify = notify.clone();
        let incoming = incoming.clone();
        task::spawn(
            handle_incoming(sending_conn, remote, notify, incoming, data),
        );
    }

    info!("rmc disconnected")
}

pub async fn new_rmc_gateway_connection<T: RmcCallable + Debug + Sync + Send + 'static, F>(
    conn: SplittableBufferConnection,
    create_internal: F,
) -> Arc<T>
where
    F: AsyncFnOnce(RmcConnection) -> Arc<T>,
{
    async move {
        let notify = Arc::new(Notify::new());
        let incoming: Arc<Mutex<HashMap<u32, RMCResponse>>> = Default::default();

        let response_recv = RmcResponseReceiver(notify.clone(), incoming.clone());

        let sending_conn = conn.duplicate_sender();

        let rmc_conn = RmcConnection(sending_conn, response_recv);

        let sending_conn = conn.duplicate_sender();

        let exposed_object = (create_internal)(rmc_conn).await;

        {
            let exposed_object = exposed_object.clone();
            tokio::spawn(async move {
                handle_incoming_loop(conn, exposed_object, notify, incoming).await;
            });

            tokio::spawn(
                async move {
                    while sending_conn.is_alive() {
                        sending_conn.send([0, 0, 0, 0, 0].to_vec()).await;
                        sleep(Duration::from_secs(10)).await;
                    }
                },
            );
        }

        exposed_object
    }
    // todo: maybe add info on who we're creating the gateway to somehow
    .await
}

define_rmc_proto! {
    proto NoProto{}
}
