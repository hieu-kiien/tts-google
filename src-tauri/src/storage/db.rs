use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotaMetrics {
    pub today_requests: u64,
    pub today_chars: u64,
    pub today_rate_limits: u64,
    pub avg_latency_ms: u64,
    pub total_requests: u64,
    pub total_chars: u64,
}

pub struct DatabaseManager {
    conn: Mutex<Connection>,
}

impl DatabaseManager {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.apply_pragmas()?;
        db.run_migrations()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory SQLite: {}", e))?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.apply_pragmas()?;
        db.run_migrations()?;
        Ok(db)
    }

    fn apply_pragmas(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA synchronous = FULL;
            PRAGMA busy_timeout = 5000;
            ",
        )
        .map_err(|e| format!("Failed to set SQLite PRAGMAs: {}", e))?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| format!("Failed to start migration transaction: {}", e))?;

        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_text TEXT NOT NULL,
                model TEXT NOT NULL,
                voice TEXT NOT NULL,
                preset TEXT NOT NULL,
                pacing TEXT NOT NULL,
                pronunciation_notes TEXT,
                output_directory TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS segments (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                text TEXT NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                audio_path TEXT,
                duration_ms INTEGER DEFAULT 0,
                error_code INTEGER,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                fingerprint TEXT,
                output_fingerprint TEXT,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                next_retry_at INTEGER,
                queued_at INTEGER,
                started_at INTEGER,
                finished_at INTEGER,
                lease_owner TEXT,
                lease_expires_at INTEGER,
                last_error_code TEXT,
                last_error_message TEXT,
                cancel_requested INTEGER NOT NULL DEFAULT 0,
                state_revision INTEGER NOT NULL DEFAULT 0,
                output_size INTEGER DEFAULT 0,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS quota_logs (
                date TEXT PRIMARY KEY,
                requests_count INTEGER NOT NULL DEFAULT 0,
                chars_count INTEGER NOT NULL DEFAULT 0,
                rate_limits_count INTEGER NOT NULL DEFAULT 0,
                total_latency_ms INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS audio_cache (
                cache_key TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                voice TEXT NOT NULL,
                file_path TEXT NOT NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                byte_size INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_accessed_at INTEGER NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to run SQLite table migrations: {}", e))?;

        // Helper migration for existing databases missing new columns
        let alter_queries = [
            "ALTER TABLE segments ADD COLUMN voice TEXT;",
            "ALTER TABLE segments ADD COLUMN fingerprint TEXT;",
            "ALTER TABLE segments ADD COLUMN output_fingerprint TEXT;",
            "ALTER TABLE segments ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0;",
            "ALTER TABLE segments ADD COLUMN next_retry_at INTEGER;",
            "ALTER TABLE segments ADD COLUMN queued_at INTEGER;",
            "ALTER TABLE segments ADD COLUMN started_at INTEGER;",
            "ALTER TABLE segments ADD COLUMN finished_at INTEGER;",
            "ALTER TABLE segments ADD COLUMN lease_owner TEXT;",
            "ALTER TABLE segments ADD COLUMN lease_expires_at INTEGER;",
            "ALTER TABLE segments ADD COLUMN last_error_code TEXT;",
            "ALTER TABLE segments ADD COLUMN last_error_message TEXT;",
            "ALTER TABLE segments ADD COLUMN cancel_requested INTEGER NOT NULL DEFAULT 0;",
            "ALTER TABLE segments ADD COLUMN state_revision INTEGER NOT NULL DEFAULT 0;",
            "ALTER TABLE segments ADD COLUMN output_size INTEGER DEFAULT 0;",
        ];

        for query in alter_queries {
            let _ = tx.execute(query, []);
        }

        tx.execute("INSERT OR REPLACE INTO schema_version (version) VALUES (1);", [])
            .map_err(|e| format!("Failed to record schema version: {}", e))?;

        tx.commit().map_err(|e| format!("Failed to commit schema migration: {}", e))?;

        info!("SQLite database migrations (schema v1) and WAL pragmas executed successfully.");
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1").map_err(|e| e.to_string())?;
        let mut rows = stmt.query([key]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let val: String = row.get(0).map_err(|e| e.to_string())?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )
        .map_err(|e| format!("Failed to save setting {}: {}", key, e))?;
        Ok(())
    }

    pub fn record_quota_metric(&self, chars: usize, is_rate_limit: bool, latency_ms: u64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let rate_inc = if is_rate_limit { 1 } else { 0 };
        let req_inc = if is_rate_limit { 0 } else { 1 };

        conn.execute(
            "INSERT INTO quota_logs (date, requests_count, chars_count, rate_limits_count, total_latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(date) DO UPDATE SET
                requests_count = requests_count + excluded.requests_count,
                chars_count = chars_count + excluded.chars_count,
                rate_limits_count = rate_limits_count + excluded.rate_limits_count,
                total_latency_ms = total_latency_ms + excluded.total_latency_ms",
            rusqlite::params![today, req_inc, chars as i64, rate_inc, latency_ms as i64],
        )
        .map_err(|e| format!("Failed to log quota metric: {}", e))?;
        Ok(())
    }

    pub fn get_quota_metrics(&self) -> Result<QuotaMetrics, String> {
        let conn = self.conn.lock().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let (today_requests, today_chars, today_rate_limits, today_total_latency): (u64, u64, u64, u64) = conn
            .query_row(
                "SELECT requests_count, chars_count, rate_limits_count, total_latency_ms
                 FROM quota_logs WHERE date = ?1",
                [&today],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap_or((0, 0, 0, 0));

        let (total_requests, total_chars): (u64, u64) = conn
            .query_row(
                "SELECT COALESCE(SUM(requests_count), 0), COALESCE(SUM(chars_count), 0) FROM quota_logs",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, 0));

        let avg_latency_ms = if today_requests > 0 {
            today_total_latency / today_requests
        } else {
            0
        };

        Ok(QuotaMetrics {
            today_requests,
            today_chars,
            today_rate_limits,
            avg_latency_ms,
            total_requests,
            total_chars,
        })
    }

    pub fn get_schema_version(&self) -> Result<i32, String> {
        let conn = self.conn.lock().unwrap();
        let ver: i32 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .map_err(|e| format!("Failed to query schema version: {}", e))?;
        Ok(ver)
    }

    pub fn recover_expired_jobs(&self) -> Result<usize, String> {
        let conn = self.conn.lock().unwrap();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let reset_count = conn.execute(
            "UPDATE segments
             SET status = 'queued',
                 lease_owner = NULL,
                 lease_expires_at = NULL,
                 state_revision = state_revision + 1
             WHERE status = 'processing'
               AND (lease_expires_at IS NULL OR lease_expires_at < ?1)",
            [now_ms],
        ).map_err(|e| format!("Failed to recover expired jobs: {}", e))?;

        if reset_count > 0 {
            info!("Recovered {} expired/orphaned processing segment jobs to 'queued'", reset_count);
        }
        Ok(reset_count)
    }

    pub fn quick_check(&self) -> Result<bool, String> {
        let conn = self.conn.lock().unwrap();
        let result: String = conn.query_row("PRAGMA quick_check;", [], |r| r.get(0))
            .map_err(|e| format!("Quick check failed: {}", e))?;
        Ok(result == "ok")
    }

    pub fn foreign_key_check(&self) -> Result<bool, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("PRAGMA foreign_key_check;").map_err(|e| e.to_string())?;
        let rows = stmt.query([]) .map_err(|e| e.to_string())?;
        // If no rows returned, foreign key integrity is clean
        let has_errors = rows.mapped(|_| Ok(())).next().is_some();
        Ok(!has_errors)
    }

    pub fn with_conn<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&Connection) -> SqlResult<R>,
    {
        let conn = self.conn.lock().unwrap();
        f(&conn).map_err(|e| format!("Database query failed: {}", e))
    }

    pub fn with_conn_mut<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Connection) -> SqlResult<R>,
    {
        let mut conn = self.conn.lock().unwrap();
        f(&mut conn).map_err(|e| format!("Database mutation failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_init_in_memory() {
        let db = DatabaseManager::in_memory().expect("Should init in-memory DB");
        db.with_conn(|conn| {
            let count: i64 = conn.query_row("SELECT count(*) FROM projects", [], |r| r.get(0))?;
            assert_eq!(count, 0);
            Ok(())
        })
        .expect("Query should succeed");
    }

    #[test]
    fn test_integrity_checks() {
        let db = DatabaseManager::in_memory().expect("Should init in-memory DB");
        assert!(db.quick_check().unwrap());
        assert!(db.foreign_key_check().unwrap());
    }

    #[test]
    fn test_schema_version_tracking() {
        let db = DatabaseManager::in_memory().expect("Should init in-memory DB");
        assert_eq!(db.get_schema_version().unwrap(), 1);
    }
}
