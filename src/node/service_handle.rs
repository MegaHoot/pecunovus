use tokio::sync::watch;
use tokio::task::JoinHandle;
use anyhow::Result;

/// Holds running tasks and shutdown channel for the node.
/// Call `shutdown()` to gracefully stop services.
pub struct ServiceHandle {
    shutdown_tx: watch::Sender<bool>,
    join_handles: Vec<JoinHandle<anyhow::Result<()>>>,
}

impl ServiceHandle {
    /// Create a new ServiceHandle and return it together with a Receiver clonable by tasks.
    pub fn new() -> (Self, watch::Receiver<bool>) {
        let (tx, rx) = watch::channel(false);
        let handle = ServiceHandle { shutdown_tx: tx, join_handles: vec![] };
        (handle, rx)
    }

    /// Attach a background task handle (so we wait on it on shutdown).
    pub fn attach(&mut self, h: JoinHandle<anyhow::Result<()>>) {
        self.join_handles.push(h);
    }

    /// Signal shutdown to all tasks and await them sequentially.
    pub async fn shutdown(mut self) -> Result<()> {
        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Wait for tasks to complete
        for h in self.join_handles {
            match h.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::error!("service task returned error: {:?}", e),
                Err(e) => tracing::error!("task join error: {:?}", e),
            }
        }
        Ok(())
    }

    /// Return a cloneable shutdown receiver for tasks that need to observe shutdown state.
    pub fn shutdown_rx(&self) -> watch::Receiver<bool> {
        self.shutdown_tx.subscribe()
    }

    /// Return the shutdown sender so external callers can signal shutdown.
    pub fn shutdown_sender(&self) -> watch::Sender<bool> {
        self.shutdown_tx.clone()
    }
}
