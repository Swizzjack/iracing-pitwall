//! Receives session-end signals from the blocking SDK loop and debounces
//! them before dispatching to the fetch worker.
//!
//! Why debounce? iRacing marks checkered before officially finalizing
//! results in the Data API. A 30-second delay gives the backend time to
//! publish the result; the retry queue handles remaining 404s.

use std::time::Duration;

use tokio::sync::mpsc;

use super::SubSessionEnd;

pub async fn run(mut finish_rx: mpsc::Receiver<SubSessionEnd>, fetch_tx: mpsc::Sender<i64>) {
    while let Some(event) = finish_rx.recv().await {
        let sub_id = event.sub_session_id;
        let tx = fetch_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            log::info!("finalizer: dispatching fetch for subsession {sub_id}");
            if tx.send(sub_id).await.is_err() {
                log::warn!("finalizer: fetch channel closed");
            }
        });
    }
}
