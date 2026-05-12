//! SQLite schema migrations for the results database.

use anyhow::{Context, Result};
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE sessions (
        sub_session_id  INTEGER PRIMARY KEY,
        session_id      INTEGER,
        series_id       INTEGER,
        series_name     TEXT,
        season_name     TEXT,
        season_year     INTEGER,
        season_quarter  INTEGER,
        track_id        INTEGER,
        track_name      TEXT,
        track_config    TEXT,
        event_type      INTEGER,
        event_type_name TEXT,
        start_time      INTEGER NOT NULL,
        end_time        INTEGER,
        weather_summary TEXT,
        sof             INTEGER,
        cars_json       TEXT,
        raw_json        TEXT NOT NULL
    );
    CREATE INDEX idx_sessions_start  ON sessions(start_time DESC);
    CREATE INDEX idx_sessions_track  ON sessions(track_id, start_time DESC);
    CREATE INDEX idx_sessions_series ON sessions(series_id, start_time DESC);
    CREATE INDEX idx_sessions_event  ON sessions(event_type, start_time DESC);

    CREATE TABLE session_results (
        sub_session_id    INTEGER,
        simsession_type   TEXT,
        cust_id           INTEGER,
        display_name      TEXT,
        finish_position   INTEGER,
        starting_position INTEGER,
        laps_complete     INTEGER,
        incidents         INTEGER,
        best_lap_ms       INTEGER,
        average_lap_ms    INTEGER,
        oldi_rating       INTEGER,
        newi_rating       INTEGER,
        oldsr             INTEGER,
        newsr             INTEGER,
        car_id            INTEGER,
        car_name          TEXT,
        car_class_id      INTEGER,
        car_class_name    TEXT,
        is_player         INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (sub_session_id, simsession_type, cust_id),
        FOREIGN KEY (sub_session_id) REFERENCES sessions(sub_session_id) ON DELETE CASCADE
    );
    CREATE INDEX idx_results_car    ON session_results(car_id);
    CREATE INDEX idx_results_player ON session_results(is_player, sub_session_id);

    CREATE TABLE pending_fetches (
        sub_session_id INTEGER PRIMARY KEY,
        next_retry_at  INTEGER NOT NULL,
        attempts       INTEGER NOT NULL DEFAULT 0,
        last_error     TEXT
    );

    CREATE TABLE meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    INSERT INTO meta VALUES ('schema_version', '1');
    "#,
];

pub fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let version: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='meta'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let current: i64 = if version > 0 {
        conn.query_row("SELECT value FROM meta WHERE key='schema_version'", [], |r| {
            r.get::<_, String>(0)
        })
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
    } else {
        0
    };

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let migration_version = (i + 1) as i64;
        if migration_version > current {
            conn.execute_batch(sql)
                .with_context(|| format!("migration v{migration_version}"))?;
            conn.execute(
                "INSERT OR REPLACE INTO meta VALUES ('schema_version', ?1)",
                [migration_version.to_string()],
            )?;
            log::info!("db: migrated to schema v{migration_version}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        apply_migrations(&conn).unwrap(); // second call must not fail
        let v: String = conn
            .query_row("SELECT value FROM meta WHERE key='schema_version'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len().to_string());
    }
}
