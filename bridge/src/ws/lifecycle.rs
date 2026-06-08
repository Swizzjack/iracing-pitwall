//! WS connection counter and auto-shutdown logic.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{oneshot, watch};

/// Shared connection counter. Cloneable; all clones share the same `watch::Sender`.
#[derive(Clone)]
pub struct ClientTracker {
    tx: Arc<watch::Sender<usize>>,
}

/// RAII guard: increments on creation, decrements on drop.
pub struct ClientGuard {
    tx: Arc<watch::Sender<usize>>,
}

impl ClientTracker {
    pub fn new() -> (Self, watch::Receiver<usize>) {
        let (tx, rx) = watch::channel(0usize);
        (Self { tx: Arc::new(tx) }, rx)
    }

    pub fn guard(&self) -> ClientGuard {
        self.tx.send_modify(|n| *n += 1);
        ClientGuard { tx: self.tx.clone() }
    }
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.tx.send_modify(|n| *n = n.saturating_sub(1));
    }
}

pub struct LifecycleConfig {
    /// If true: no auto-shutdown (useful for headless tests).
    pub keep_alive: bool,
    /// How long the count must stay at 0 before shutdown is triggered.
    pub grace: Duration,
    /// How long to wait before the bridge exits if no browser ever connects.
    pub startup_grace: Duration,
}

/// Spawn this function as its own task. Sends once on `shutdown_tx` when
/// the bridge should terminate.
pub async fn run_watcher(
    mut rx: watch::Receiver<usize>,
    cfg: LifecycleConfig,
    shutdown_tx: oneshot::Sender<()>,
) {
    if cfg.keep_alive {
        return;
    }

    // Phase 1: wait for the first connection. Timeout = startup_grace.
    tokio::select! {
        r = rx.wait_for(|n| *n >= 1) => {
            if r.is_err() { return; } // Sender dropped
        }
        _ = tokio::time::sleep(cfg.startup_grace) => {
            log::warn!(
                "no client connected within {}s — shutting down",
                cfg.startup_grace.as_secs()
            );
            let _ = shutdown_tx.send(());
            return;
        }
    }

    // Phase 2: watch the count. If it drops to 0, start the grace timer.
    loop {
        if rx.wait_for(|n| *n == 0).await.is_err() {
            return; // Sender dropped, the bridge is shutting down anyway
        }
        tokio::select! {
            _ = tokio::time::sleep(cfg.grace) => {
                log::info!(
                    "no clients for {}s — shutting down",
                    cfg.grace.as_secs()
                );
                let _ = shutdown_tx.send(());
                return;
            }
            r = rx.wait_for(|n| *n >= 1) => {
                if r.is_err() { return; }
                // A client reconnected — keep going
            }
        }
    }
}
