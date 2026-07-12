impl MessageDelivery for User {
    async fn deliver_message(&self, mut message: Any<UserMessage>) -> Result<(), ErrorCode> {
        let mut msg = message.get()?;

        let _users = match msg.recipient_type {
            1 => {
                let Some(user) = self
                    .matchmake_manager
                    .users_by_pid
                    .read()
                    .await
                    .get(&msg.recipient_id)
                    .map(Weak::upgrade)
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
