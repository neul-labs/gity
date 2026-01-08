use async_nng::AsyncSocket;
use bincode::Options as BincodeOptions;
use gity_ipc::{bounded_bincode, validate_message_size, DaemonNotification};
use nng::{
    options::{protocol::pubsub::Subscribe, Options},
    Protocol,
};
use tokio::sync::mpsc;

use crate::{map_client_error, DaemonError, ServerError, Shutdown};

/// Publishes daemon notifications over a PUB socket.
pub struct NotificationServer {
    address: String,
    receiver: mpsc::UnboundedReceiver<DaemonNotification>,
}

impl NotificationServer {
    pub fn new(
        address: impl Into<String>,
        receiver: mpsc::UnboundedReceiver<DaemonNotification>,
    ) -> Self {
        Self {
            address: address.into(),
            receiver,
        }
    }

    pub async fn run(mut self, shutdown: Shutdown) -> Result<(), ServerError> {
        let socket = nng::Socket::new(Protocol::Pub0)?;
        socket.listen(&self.address)?;
        let mut async_socket = AsyncSocket::try_from(socket)?;

        loop {
            tokio::select! {
                _ = shutdown.wait() => break,
                message = self.receiver.recv() => match message {
                    Some(notification) => {
                        let payload = bounded_bincode()
                            .serialize(&notification)
                            .map_err(|err| ServerError::Serialization(err.to_string()))?;
                        let mut msg = nng::Message::new();
                        msg.push_back(&payload);
                        async_socket
                            .send(msg, None)
                            .await
                            .map_err(|(_, err)| ServerError::Socket(err))?;
                    }
                    None => break,
                }
            }
        }

        Ok(())
    }
}

/// Subscribes to daemon notifications via SUB sockets.
pub struct NotificationSubscriber {
    address: String,
}

pub struct NotificationStream {
    socket: AsyncSocket,
}

impl NotificationSubscriber {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    pub async fn connect(&self) -> Result<NotificationStream, DaemonError> {
        let socket = nng::Socket::new(Protocol::Sub0).map_err(map_client_error)?;
        socket
            .set_opt::<Subscribe>(Vec::new())
            .map_err(map_client_error)?;
        socket.dial(&self.address).map_err(map_client_error)?;
        let async_socket = AsyncSocket::try_from(socket).map_err(map_client_error)?;
        Ok(NotificationStream {
            socket: async_socket,
        })
    }
}

impl NotificationStream {
    pub async fn next(&mut self) -> Result<DaemonNotification, DaemonError> {
        let message = self.socket.receive(None).await.map_err(map_client_error)?;
        let data = message.as_slice();

        validate_message_size(data).map_err(|err| DaemonError::Transport(err.to_string()))?;

        let notification: DaemonNotification = bounded_bincode()
            .deserialize(data)
            .map_err(|err| DaemonError::Transport(err.to_string()))?;

        Ok(notification)
    }
}
