use crate::MessagingManager;
use rnex_msg_protos::message_delivery::RemoteMessageDeliveryNoResponse;
use rnex_msg_protos::{
    LocalMessagingProtocol, RemoteMessagingClient, message_delivery::MessageDelivery,
    messaging::UserMessage,
};
use rnex_rmc::{any::Any, response::ErrorCode, rmc_struct, util::PID};
use rnex_server::{PassthroughInitModule, WeakPassthroughInitModule};

#[rmc_struct(MessagingProtocol)]
pub struct MessagingUser {
    pub msgm: PassthroughInitModule<MessagingManager>,
    pub pid: PID,
    pub remote: RemoteMessagingClient,
}

impl MessageDelivery for MessagingUser {
    async fn deliver_message(&self, mut message: Any<UserMessage>) -> Result<(), ErrorCode> {
        let mut msg = message.get()?;

        let _users = match msg.recipient_type {
            1 => {
                let Some(user) = self
                    .msgm
                    .users_by_pid
                    .read()
                    .await
                    .get(&msg.recipient_id)
                    .map(WeakPassthroughInitModule::upgrade)
                    .flatten()
                else {
                    return Err(ErrorCode::Core_InvalidArgument);
                };
                if msg.flags & 1 != 0 {
                    msg.recipient_id = user.pid;
                    msg.recipient_type = 1;
                }

                message.emplace_parent(&msg)?;

                user.remote.deliver_message(message).await;
            }
            2 => {
                return Err(ErrorCode::Core_NotImplemented);
            }
            _ => {
                return Err(ErrorCode::Core_InvalidArgument);
            }
        };

        Err(ErrorCode::Core_NotImplemented)
    }
}
