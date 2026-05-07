//! WS-Connection-Counter und Auto-Shutdown-Logik.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{oneshot, watch};

/// Shared connection counter. Clone-able; alle Klone teilen denselben `watch::Sender`.
#[derive(Clone)]
pub struct ClientTracker {
    tx: Arc<watch::Sender<usize>>,
}

/// RAII-Guard: inkrementiert beim Erstellen, dekrementiert beim Drop.
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
    /// Wenn true: kein Auto-Shutdown (nützlich für headless-Tests).
    pub keep_alive: bool,
    /// Zeit, die der Count auf 0 bleiben muss, bevor Shutdown ausgelöst wird.
    pub grace: Duration,
    /// Zeit, nach der sich die Bridge beendet wenn sich kein Browser verbindet.
    pub startup_grace: Duration,
}

/// Spawne diese Funktion als eigenen Task. Sendet einmal auf `shutdown_tx` wenn
/// die Bridge beendet werden soll.
pub async fn run_watcher(
    mut rx: watch::Receiver<usize>,
    cfg: LifecycleConfig,
    shutdown_tx: oneshot::Sender<()>,
) {
    if cfg.keep_alive {
        return;
    }

    // Phase 1: Warte auf erste Verbindung. Timeout = startup_grace.
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

    // Phase 2: Beobachte count. Wenn er auf 0 fällt, starte Grace-Timer.
    loop {
        if rx.wait_for(|n| *n == 0).await.is_err() {
            return; // Sender dropped, Bridge fährt sowieso herunter
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
                // Client hat sich neu verbunden — weitermachen
            }
        }
    }
}
