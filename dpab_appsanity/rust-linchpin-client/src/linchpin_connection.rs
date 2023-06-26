use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{debug, error};

use crate::{
    model::client_messages::LinchpinClientError,
    model::linchpin_messages::{Message, MessageType},
};

pub struct LinchpinConnection {
    linchpin_message_tx: SplitSink<
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
    linchpin_message_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    linchpin_request_rx: mpsc::Receiver<Message>,
    linchpin_response_tx: mpsc::Sender<MessageType>,
}

pub async fn start(
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    linchpin_request_rx: mpsc::Receiver<Message>,
    linchpin_response_tx: mpsc::Sender<MessageType>,
) {
    LinchpinConnection::new(socket, linchpin_request_rx, linchpin_response_tx)
        .handle_messages()
        .await;
}

impl LinchpinConnection {
    fn new(
        socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
        linchpin_request_rx: mpsc::Receiver<Message>,
        linchpin_response_tx: mpsc::Sender<MessageType>,
    ) -> LinchpinConnection {
        let (linchpin_sender, linchpin_receiver) = socket.split();
        LinchpinConnection {
            linchpin_message_tx: linchpin_sender,
            linchpin_message_rx: linchpin_receiver,
            linchpin_request_rx,
            linchpin_response_tx,
        }
    }

    async fn handle_linchpin_message(
        &mut self,
        message: Option<
            Result<tokio_tungstenite::tungstenite::Message, tokio_tungstenite::tungstenite::Error>,
        >,
    ) -> Result<(), LinchpinClientError> {
        debug!(
            "linchpin_connection: handle_linchpin_message: message={:?}",
            message
        );
        if let None = message {
            return Err(LinchpinClientError::IoError);
        }

        let message = message.unwrap();
        if let Err(e) = message {
            error!("linchpin_connection: handle_linchpin_message: e={}", e);
            return Err(LinchpinClientError::IoError);
        }

        let text = message.unwrap().into_text();
        if let Err(e) = text {
            error!(
                "linchpin_connection: handle_linchpin_message: Invalid message format: e={}",
                e
            );
            return Ok(());
        }

        let message_type: Result<MessageType, serde_json::Error> =
            serde_json::from_str(&text.unwrap());
        if let Err(e) = message_type {
            error!(
                "linchpin_connection: handle_linchpin_message: Unknown message type: e={}",
                e
            );
            return Ok(());
        }

        if let Err(e) = self.linchpin_response_tx.send(message_type.unwrap()).await {
            error!(
                "linchpin_connection: handle_linchpin_message: Could not send response: e={:?}",
                e
            );
        }
        Ok(())
    }

    async fn handle_messages(&mut self) {
        loop {
            tokio::select! {
                from_linchpin = self.linchpin_message_rx.next() => {
                    if let Err(_) = self.handle_linchpin_message(from_linchpin).await {
                        break;
                    }
                }
                to_linchpin = self.linchpin_request_rx.recv() => {
                    if let None = to_linchpin {
                        break;
                    }
                    self.send_to_linchpin(to_linchpin).await;
                }
            }
        }
        debug!("handle_messages: Exiting from_linchpin/to_linchpin thread");
        if let Err(e) = self
            .linchpin_response_tx
            .send(MessageType::Disconnected)
            .await
        {
            error!("handle_messages: Could not send response: e={:?}", e);
        }
    }

    pub async fn send_to_linchpin(&mut self, message: Option<Message>) {
        debug!(
            "linchpin_connection: send_to_linchpin: message={:?}",
            message
        );
        let sink_message = tokio_tungstenite::tungstenite::Message::text(
            serde_json::to_string(&message.unwrap()).unwrap(),
        );
        if let Err(e) = self.linchpin_message_tx.send(sink_message).await {
            error!("linchpin_connection: send_to_linchpin: Send error: e={}", e);
        }
    }
}
