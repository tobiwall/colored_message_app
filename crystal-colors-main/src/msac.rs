use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::sync::RwLock;

/// A channel, where a message is forwarded to all connected clients.
pub struct Channel {
    tx: tokio::sync::mpsc::Sender<String>,
    connections: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<String>>>>,
    num_connections: AtomicUsize,
}

impl Channel {
    pub fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let outer_connections: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<String>>>> =
            Arc::new(RwLock::new(Vec::new()));
        let connections = outer_connections.clone();
        tokio::spawn(async move {
            loop {
                let message: String = match rx.recv().await {
                    Some(message) => message,
                    None => break,
                };
                let connections_read = connections.read().await;
                let mut to_remove = Vec::new();
                for (i, tx) in connections_read.iter().enumerate() {
                    if (tx.send(message.clone()).await).is_err() {
                        to_remove.push(i);
                    }
                }
                if !to_remove.is_empty() {
                    drop(connections_read);
                    let mut connections_write = connections.write().await;
                    for i in to_remove.into_iter().rev() {
                        connections_write.remove(i);
                    }
                }
            }
        });
        Self {
            tx,
            connections: outer_connections,
            num_connections: AtomicUsize::new(0),
        }
    }

    pub async fn add(
        &self,
    ) -> (
        tokio::sync::mpsc::Sender<String>,
        tokio::sync::mpsc::Receiver<String>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut connections = self.connections.write().await;
        connections.push(tx);
        self.num_connections.fetch_add(1, Ordering::Relaxed);
        (self.tx.clone(), rx)
    }

    pub async fn remove(&self) -> bool {
        self.num_connections.fetch_sub(1, Ordering::Relaxed) == 1
    }
}
