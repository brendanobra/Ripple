use tokio::sync::{mpsc, oneshot};
use tracing::{debug_span, error_span, instrument, Span};

#[instrument(skip(tx))]
pub async fn mpsc_send_and_log<T: std::fmt::Debug>(
    tx: &mpsc::Sender<T>,
    message: T,
    channel_id: &str,
) {
    match tx.send(message).await {
        Ok(_) => {
            let _span = debug_span!(
                parent: Span::current(),
                "Successfully sent message through mpsc channel",
                channel_id
            );
            ()
        }
        Err(e) => {
            let _span = error_span!(
                parent: Span::current(),
                "Failed to send message through mpsc channel",
                channel_id,
                ?e
            );
            ()
        }
    }
}

#[instrument(skip(tx))]
pub fn oneshot_send_and_log<T: std::fmt::Debug>(
    tx: oneshot::Sender<T>,
    message: T,
    channel_id: &str,
) {
    match tx.send(message) {
        Ok(_) => {
            let _span = debug_span!(
                parent: Span::current(),
                "Successfully sent message through oneshot channel",
                channel_id
            );
            ()
        }
        Err(e) => {
            let _span = error_span!(
                parent: Span::current(),
                "Failed to send message through oneshot channel",
                channel_id,
                ?e
            );
            ()
        }
    }
}
