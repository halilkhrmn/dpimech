use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, oneshot};

use super::{
    ClientFrame, Event, Reply, Request, ServerFrame, frame_lines, read_frame, write_frame,
};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("not connected to the DPIMech service")]
    Disconnected,
    #[error("{0}")]
    Service(String),
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Reply, String>>>>>;

/// Cheap to clone; all clones share one connection.
#[derive(Clone)]
pub struct Client {
    tx: mpsc::UnboundedSender<(Request, oneshot::Sender<Result<Reply, String>>)>,
}

impl Client {
    /// Connects to the service. Events arrive on the returned receiver, which
    /// closes when the connection drops.
    pub async fn connect() -> std::io::Result<(Client, mpsc::UnboundedReceiver<Event>)> {
        let stream = super::connect().await?;
        let (reader, mut writer) = tokio::io::split(stream);
        let pending: Pending = Arc::default();
        let (tx, mut rx) = mpsc::unbounded_channel::<(Request, oneshot::Sender<_>)>();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let writer_pending = pending.clone();
        tokio::spawn(async move {
            let mut next_id = 0u64;
            while let Some((request, reply_tx)) = rx.recv().await {
                next_id += 1;
                writer_pending.lock().unwrap().insert(next_id, reply_tx);
                let frame = ClientFrame {
                    id: next_id,
                    request,
                };
                if write_frame(&mut writer, &frame).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut lines = frame_lines(reader);
            while let Ok(Some(frame)) = read_frame::<_, ServerFrame>(&mut lines).await {
                match frame {
                    ServerFrame::Reply { id, result } => {
                        if let Some(reply_tx) = pending.lock().unwrap().remove(&id) {
                            let _ = reply_tx.send(result);
                        }
                    }
                    ServerFrame::Event { event } => {
                        let _ = event_tx.send(event);
                    }
                }
            }
            // Dropping pending senders fails every in-flight request with Disconnected.
            pending.lock().unwrap().clear();
        });

        Ok((Client { tx }, event_rx))
    }

    pub async fn request(&self, request: Request) -> Result<Reply, ClientError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send((request, reply_tx))
            .map_err(|_| ClientError::Disconnected)?;
        reply_rx
            .await
            .map_err(|_| ClientError::Disconnected)?
            .map_err(ClientError::Service)
    }
}
