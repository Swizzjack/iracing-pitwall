//! SQLite persistence layer for race results.

pub mod queries;
pub mod schema;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use rusqlite::Connection;
use tokio::sync::Mutex;

/// Thread-safe wrapper around a rusqlite connection.
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &PathBuf) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("open db at {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        schema::apply_migrations(&conn)?;
        log::info!("db: opened {}", path.display());
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    /// Run a blocking database operation inside a `spawn_blocking` task.
    pub async fn with<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let arc = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let guard = arc.blocking_lock();
            f(&guard)
        })
        .await
        .context("db task panicked")?
    }
}
