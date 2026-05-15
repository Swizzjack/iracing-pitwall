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
    // v2 — live-capture columns + rebuilt session_results (new PK with car_idx) + live_session_laps
    r#"
    ALTER TABLE sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'live';
    ALTER TABLE sessions ADD COLUMN captured_at INTEGER;

    DROP INDEX IF EXISTS idx_results_car;
    DROP INDEX IF EXISTS idx_results_player;

    CREATE TABLE session_results_new (
        sub_session_id    INTEGER NOT NULL,
        simsession_type   TEXT    NOT NULL DEFAULT '',
        car_idx           INTEGER NOT NULL DEFAULT -1,
        cust_id           INTEGER,
        display_name      TEXT,
        finish_position   INTEGER,
        starting_position INTEGER,
        class_position    INTEGER,
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
        source            TEXT    NOT NULL DEFAULT 'live',
        last_sectors_json TEXT,
        best_sectors_json TEXT,
        pit_stops         INTEGER,
        tire_compound     INTEGER,
        car_number        TEXT,
        safety_rating     TEXT,
        lic_color         INTEGER,
        car_class_color   INTEGER,
        reason_out        TEXT,
        PRIMARY KEY (sub_session_id, simsession_type, car_idx),
        FOREIGN KEY (sub_session_id) REFERENCES sessions(sub_session_id) ON DELETE CASCADE
    );

    INSERT INTO session_results_new (
        sub_session_id, simsession_type, car_idx, cust_id,
        display_name, finish_position, starting_position,
        laps_complete, incidents, best_lap_ms, average_lap_ms,
        oldi_rating, newi_rating, oldsr, newsr,
        car_id, car_name, car_class_id, car_class_name,
        is_player, source
    )
    SELECT
        sub_session_id, simsession_type,
        (-1 - rowid) AS car_idx,
        cust_id,
        display_name, finish_position, starting_position,
        laps_complete, incidents, best_lap_ms, average_lap_ms,
        oldi_rating, newi_rating, oldsr, newsr,
        car_id, car_name, car_class_id, car_class_name,
        is_player, 'api'
    FROM session_results;

    DROP TABLE session_results;
    ALTER TABLE session_results_new RENAME TO session_results;

    CREATE INDEX idx_results_car    ON session_results(car_id);
    CREATE INDEX idx_results_player ON session_results(is_player, sub_session_id);

    CREATE TABLE live_session_laps (
        sub_session_id  INTEGER NOT NULL,
        simsession_num  INTEGER NOT NULL,
        car_idx         INTEGER NOT NULL,
        lap_num         INTEGER NOT NULL,
        cust_id         INTEGER,
        lap_time_sec    REAL,
        sectors_json    TEXT,
        valid           INTEGER NOT NULL DEFAULT 1,
        in_lap          INTEGER NOT NULL DEFAULT 0,
        out_lap         INTEGER NOT NULL DEFAULT 0,
        incidents_delta INTEGER,
        position_at_end INTEGER,
        air_temp        REAL,
        track_temp      REAL,
        track_wetness   INTEGER,
        rubber_state    INTEGER,
        session_time    REAL,
        PRIMARY KEY (sub_session_id, simsession_num, car_idx, lap_num),
        FOREIGN KEY (sub_session_id) REFERENCES sessions(sub_session_id) ON DELETE CASCADE
    );
    CREATE INDEX idx_laps_car ON live_session_laps(sub_session_id, car_idx);

    INSERT OR REPLACE INTO meta VALUES ('schema_version', '2');
    "#,
    // v3 — session_segments: per-phase metadata split from sessions
    r#"
    CREATE TABLE session_segments (
        sub_session_id   INTEGER NOT NULL,
        simsession_type  TEXT    NOT NULL,
        simsession_num   INTEGER NOT NULL DEFAULT 0,
        event_type_name  TEXT,
        start_time       INTEGER NOT NULL,
        end_time         INTEGER,
        weather_summary  TEXT,
        sof              INTEGER,
        raw_json         TEXT,
        source           TEXT    NOT NULL DEFAULT 'live',
        captured_at      INTEGER,
        is_finalized     INTEGER NOT NULL DEFAULT 1,
        PRIMARY KEY (sub_session_id, simsession_type),
        FOREIGN KEY (sub_session_id) REFERENCES sessions(sub_session_id) ON DELETE CASCADE
    );
    CREATE INDEX idx_segments_start ON session_segments(start_time DESC);

    INSERT OR IGNORE INTO session_segments
        (sub_session_id, simsession_type, simsession_num, event_type_name,
         start_time, end_time, weather_summary, sof, raw_json, source, captured_at, is_finalized)
    SELECT DISTINCT
        sr.sub_session_id,
        sr.simsession_type,
        0,
        s.event_type_name,
        s.start_time,
        s.end_time,
        s.weather_summary,
        s.sof,
        s.raw_json,
        COALESCE(s.source, 'live'),
        s.captured_at,
        1
    FROM session_results sr
    JOIN sessions s ON s.sub_session_id = sr.sub_session_id;

    INSERT OR IGNORE INTO session_segments
        (sub_session_id, simsession_type, simsession_num, event_type_name,
         start_time, end_time, weather_summary, sof, raw_json, source, captured_at, is_finalized)
    SELECT s.sub_session_id, 'R', 0, s.event_type_name,
           s.start_time, s.end_time, s.weather_summary, s.sof, s.raw_json,
           COALESCE(s.source, 'live'), s.captured_at, 1
    FROM sessions s
    WHERE NOT EXISTS (SELECT 1 FROM session_segments ss WHERE ss.sub_session_id = s.sub_session_id);

    INSERT OR REPLACE INTO meta VALUES ('schema_version', '3');
    "#,
    // v4 — normalise all timestamps to milliseconds (previously stored as seconds).
    // Any value < 10_000_000_000 is assumed to be seconds; multiply by 1000.
    r#"
    UPDATE sessions SET start_time  = start_time  * 1000 WHERE start_time  > 0   AND start_time  < 10000000000;
    UPDATE sessions SET end_time    = end_time    * 1000 WHERE end_time    IS NOT NULL AND end_time    < 10000000000;
    UPDATE sessions SET captured_at = captured_at * 1000 WHERE captured_at IS NOT NULL AND captured_at < 10000000000;
    UPDATE session_segments SET start_time  = start_time  * 1000 WHERE start_time  > 0   AND start_time  < 10000000000;
    UPDATE session_segments SET end_time    = end_time    * 1000 WHERE end_time    IS NOT NULL AND end_time    < 10000000000;
    UPDATE session_segments SET captured_at = captured_at * 1000 WHERE captured_at IS NOT NULL AND captured_at < 10000000000;
    UPDATE pending_fetches SET next_retry_at = next_retry_at * 1000 WHERE next_retry_at < 10000000000;
    INSERT OR REPLACE INTO meta VALUES ('schema_version', '4');
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
