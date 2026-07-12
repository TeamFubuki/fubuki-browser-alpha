use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use frost_protocol::{BookmarkRecord, DownloadRecord, HistoryRecord, PermissionRecord};
use rusqlite::{Connection, ffi, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid setting key: {0}")]
    InvalidKey(String),
    #[error("invalid value for setting {key}: {value}")]
    InvalidValue { key: String, value: String },
}

pub type StoreResult<T> = Result<T, StoreError>;

pub trait SettingsRepository {
    fn get_setting(&self, key: &str) -> StoreResult<Option<String>>;
    fn set_setting(&self, key: &str, value: &str) -> StoreResult<()>;
    fn remove_setting(&self, key: &str) -> StoreResult<()>;
}

pub trait BookmarkRepository {
    fn list_bookmarks(&self) -> StoreResult<Vec<BookmarkRecord>>;
    fn save_bookmark(&self, title: &str, url: &str, favicon_url: &str) -> StoreResult<bool>;
    fn remove_bookmark(&self, url: &str) -> StoreResult<bool>;
}

pub trait HistoryRepository {
    fn list_history(&self) -> StoreResult<Vec<HistoryRecord>>;
    fn add_history(&self, title: &str, url: &str, favicon_url: &str) -> StoreResult<()>;
    fn remove_history(&self, url: &str) -> StoreResult<bool>;
    fn clear_history_range(&self, range: &str) -> StoreResult<bool>;
}

pub trait DownloadRepository {
    fn list_downloads(&self) -> StoreResult<Vec<DownloadRecord>>;
    fn upsert_download(&self, url: &str, path: &str, state: &str, percent: i64) -> StoreResult<()>;
    fn remove_download(&self, url: Option<&str>, path: Option<&str>) -> StoreResult<bool>;
}

pub trait PermissionRepository {
    fn list_permissions(&self) -> StoreResult<Vec<PermissionRecord>>;
    fn set_permission(&self, origin: &str, permission: &str, value: &str) -> StoreResult<()>;
    fn remove_permission(&self, origin: &str, permission: &str) -> StoreResult<bool>;
}

/// Application log storage. Native host writes diagnostic logs here so that
/// they survive in the engine-owned SQLite database rather than a second,
/// host-owned store.
pub trait LogRepository {
    fn add_log(&self, level: &str, message: &str) -> StoreResult<()>;
    fn list_logs(&self, limit: usize) -> StoreResult<Vec<LogRecord>>;
    fn clear_logs(&self) -> StoreResult<()>;
}

/// Session snapshot persistence (window/tab layout for restore on launch).
pub trait SessionRepository {
    fn get_session(&self) -> StoreResult<Option<String>>;
    fn set_session(&self, json: &str) -> StoreResult<()>;
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecord {
    pub level: String,
    pub message: String,
    pub created_at: String,
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        // When multiple threads race to open the same database file the
        // initial Connection::open can fail with "database is locked"
        // because the busy-timeout has not been configured yet. Retry a
        // handful of times with exponential back-off to let the first
        // connection finish creating the WAL / SHM files.
        let mut last_err = None;
        for attempt in 0..32 {
            match Connection::open(&path) {
                Ok(conn) => {
                    let mut store = Self { conn };
                    // Set busy_timeout *before* migrate so concurrent
                    // writers inside migrate() will wait instead of failing.
                    store.conn.busy_timeout(Duration::from_secs(10))?;
                    store.configure_connection(ConnectionKind::Persistent)?;
                    store.migrate()?;
                    return Ok(store);
                }
                Err(rusqlite::Error::SqliteFailure(err, msg))
                    if err.code == ffi::ErrorCode::DatabaseBusy =>
                {
                    last_err = Some(rusqlite::Error::SqliteFailure(err, msg));
                    // Exponential backoff: 10ms, 20ms, 40ms, ... up to ~5 seconds total
                    std::thread::sleep(Duration::from_millis(10 * 2u64.pow(attempt.min(9) as u32)));
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(last_err.unwrap().into())
    }

    pub fn in_memory() -> StoreResult<Self> {
        let mut store = Self {
            conn: Connection::open_in_memory()?,
        };
        store.configure_connection(ConnectionKind::Ephemeral)?;
        store.migrate()?;
        Ok(store)
    }

    fn configure_connection(&self, kind: ConnectionKind) -> StoreResult<()> {
        // The engine worker and the native host intentionally use separate
        // connections to the same profile database. Wait through brief write
        // contention and use WAL so reads do not block normal browser writes.
        self.conn.busy_timeout(Duration::from_secs(5))?;
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        match kind {
            ConnectionKind::Persistent => self.conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;",
            )?,
            ConnectionKind::Ephemeral => self.conn.execute_batch("PRAGMA synchronous = FULL;")?,
        }
        Ok(())
    }

    fn migrate(&mut self) -> StoreResult<()> {
        let transaction = self.conn.transaction()?;
        transaction.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY NOT NULL,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS bookmarks (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              title TEXT NOT NULL,
              url TEXT NOT NULL UNIQUE,
              favicon_url TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS history (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              title TEXT NOT NULL,
              url TEXT NOT NULL,
              favicon_url TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS downloads (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              url TEXT NOT NULL DEFAULT '',
              path TEXT NOT NULL DEFAULT '',
              state TEXT NOT NULL,
              percent INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS permissions (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              origin TEXT NOT NULL,
              permission TEXT NOT NULL,
              value TEXT NOT NULL,
              created_at TEXT NOT NULL,
              UNIQUE(origin, permission)
            );
            CREATE TABLE IF NOT EXISTS logs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              level TEXT NOT NULL,
              message TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              snapshot TEXT NOT NULL
            );
            ",
        )?;
        transaction.commit()?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ConnectionKind {
    Persistent,
    Ephemeral,
}

/// Bulk-clear operations used by the `data.clear` command.
pub trait ClearRepository {
    fn clear_bookmarks(&self) -> StoreResult<()>;
    fn clear_downloads(&self) -> StoreResult<()>;
}

impl BookmarkRepository for SqliteStore {
    fn list_bookmarks(&self) -> StoreResult<Vec<BookmarkRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT title, url, favicon_url, created_at FROM bookmarks ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BookmarkRecord {
                title: row.get(0)?,
                url: row.get(1)?,
                favicon_url: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn save_bookmark(&self, title: &str, url: &str, favicon_url: &str) -> StoreResult<bool> {
        if url.is_empty() || url.starts_with("fubuki://") || url.starts_with("data:") {
            return Ok(false);
        }
        self.conn.execute(
            "
            INSERT INTO bookmarks (title, url, favicon_url, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE SET
              title = excluded.title,
              favicon_url = excluded.favicon_url
            ",
            params![empty_fallback(title, url), url, favicon_url, now_text()],
        )?;
        Ok(true)
    }

    fn remove_bookmark(&self, url: &str) -> StoreResult<bool> {
        let changed = self
            .conn
            .execute("DELETE FROM bookmarks WHERE url = ?1", params![url])?;
        Ok(changed > 0)
    }
}

impl HistoryRepository for SqliteStore {
    fn list_history(&self) -> StoreResult<Vec<HistoryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT title, url, favicon_url, created_at FROM history ORDER BY id DESC LIMIT 500",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HistoryRecord {
                title: row.get(0)?,
                url: row.get(1)?,
                favicon_url: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn add_history(&self, title: &str, url: &str, favicon_url: &str) -> StoreResult<()> {
        if url.is_empty() || url.starts_with("fubuki://") || url.starts_with("data:") {
            return Ok(());
        }
        self.conn
            .execute("DELETE FROM history WHERE url = ?1", params![url])?;
        self.conn.execute(
            "INSERT INTO history (title, url, favicon_url, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![empty_fallback(title, url), url, favicon_url, now_text()],
        )?;
        self.conn.execute(
            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT 500)",
            [],
        )?;
        Ok(())
    }

    fn remove_history(&self, url: &str) -> StoreResult<bool> {
        let changed = self
            .conn
            .execute("DELETE FROM history WHERE url = ?1", params![url])?;
        Ok(changed > 0)
    }

    fn clear_history_range(&self, range: &str) -> StoreResult<bool> {
        match range {
            "all" => {
                self.conn.execute("DELETE FROM history", [])?;
                Ok(true)
            }
            "lastHour" => {
                let cutoff = now_text().parse::<i64>().unwrap_or(0) - 3600;
                let changed = self.conn.execute(
                    "DELETE FROM history WHERE CAST(created_at AS INTEGER) >= ?1",
                    params![cutoff.to_string()],
                )?;
                Ok(changed > 0)
            }
            "today" => {
                let now = now_text().parse::<i64>().unwrap_or(0);
                let start_of_today = now - (now % 86400);
                let changed = self.conn.execute(
                    "DELETE FROM history WHERE CAST(created_at AS INTEGER) >= ?1",
                    params![start_of_today.to_string()],
                )?;
                Ok(changed > 0)
            }
            _ => Ok(false),
        }
    }
}

impl DownloadRepository for SqliteStore {
    fn list_downloads(&self) -> StoreResult<Vec<DownloadRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT url, path, state, percent, created_at FROM downloads ORDER BY id DESC LIMIT 200",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DownloadRecord {
                url: row.get(0)?,
                path: row.get(1)?,
                state: row.get(2)?,
                percent: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn upsert_download(&self, url: &str, path: &str, state: &str, percent: i64) -> StoreResult<()> {
        let now = now_text();
        let changed = self.conn.execute(
            "
            UPDATE downloads
            SET state = ?1, percent = ?2, updated_at = ?3
            WHERE url = ?4 AND path = ?5
            ",
            params![state, percent, now, url, path],
        )?;
        if changed == 0 {
            self.conn.execute(
                "
                INSERT INTO downloads (url, path, state, percent, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                ",
                params![url, path, state, percent, now],
            )?;
        }
        Ok(())
    }

    fn remove_download(&self, url: Option<&str>, path: Option<&str>) -> StoreResult<bool> {
        let changed = self.conn.execute(
            "
            DELETE FROM downloads
            WHERE (url = ?1 AND ?1 <> '') OR (path = ?2 AND ?2 <> '')
            ",
            params![url.unwrap_or_default(), path.unwrap_or_default()],
        )?;
        Ok(changed > 0)
    }
}

impl SettingsRepository for SqliteStore {
    fn get_setting(&self, key: &str) -> StoreResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1 LIMIT 1")?;
        let mut rows = stmt.query(params![key])?;
        Ok(rows
            .next()?
            .map(|row| row.get::<_, String>(0))
            .transpose()?)
    }

    fn set_setting(&self, key: &str, value: &str) -> StoreResult<()> {
        self.conn.execute(
            "
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
              value = excluded.value,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![key, value],
        )?;
        Ok(())
    }

    fn remove_setting(&self, key: &str) -> StoreResult<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }
}

impl LogRepository for SqliteStore {
    fn add_log(&self, level: &str, message: &str) -> StoreResult<()> {
        self.conn.execute(
            "INSERT INTO logs(level,message,created_at) VALUES (?1,?2,?3)",
            params![level, message, now_text()],
        )?;
        self.conn.execute(
            "DELETE FROM logs WHERE id NOT IN (SELECT id FROM logs ORDER BY id DESC LIMIT 300)",
            [],
        )?;
        Ok(())
    }

    fn list_logs(&self, limit: usize) -> StoreResult<Vec<LogRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT level,message,created_at FROM logs ORDER BY id DESC LIMIT ?")?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(LogRecord {
                level: row.get(0)?,
                message: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn clear_logs(&self) -> StoreResult<()> {
        self.conn.execute("DELETE FROM logs", [])?;
        Ok(())
    }
}

impl SessionRepository for SqliteStore {
    fn get_session(&self) -> StoreResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT snapshot FROM session WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        Ok(rows
            .next()?
            .map(|row| row.get::<_, String>(0))
            .transpose()?)
    }

    fn set_session(&self, json: &str) -> StoreResult<()> {
        self.conn.execute(
            "INSERT INTO session(id,snapshot) VALUES (1,?1) ON CONFLICT(id) DO UPDATE SET snapshot = excluded.snapshot",
            params![json],
        )?;
        Ok(())
    }
}

impl ClearRepository for SqliteStore {
    fn clear_bookmarks(&self) -> StoreResult<()> {
        self.conn.execute("DELETE FROM bookmarks", [])?;
        Ok(())
    }

    fn clear_downloads(&self) -> StoreResult<()> {
        self.conn.execute("DELETE FROM downloads", [])?;
        Ok(())
    }
}

impl PermissionRepository for SqliteStore {
    fn list_permissions(&self) -> StoreResult<Vec<PermissionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT origin, permission, value, created_at FROM permissions ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PermissionRecord {
                origin: row.get(0)?,
                permission: row.get(1)?,
                value: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    fn set_permission(&self, origin: &str, permission: &str, value: &str) -> StoreResult<()> {
        self.conn.execute(
            "
            INSERT INTO permissions (origin, permission, value, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(origin, permission) DO UPDATE SET
              value = excluded.value
            ",
            params![origin, permission, value, now_text()],
        )?;
        Ok(())
    }

    fn remove_permission(&self, origin: &str, permission: &str) -> StoreResult<bool> {
        let changed = self.conn.execute(
            "DELETE FROM permissions WHERE origin = ?1 AND permission = ?2",
            params![origin, permission],
        )?;
        Ok(changed > 0)
    }
}

fn empty_fallback<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

fn now_text() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_settings() {
        let store = SqliteStore::in_memory().unwrap();
        assert_eq!(store.get_setting("theme").unwrap(), None);

        store.set_setting("theme", "dark").unwrap();
        assert_eq!(store.get_setting("theme").unwrap(), Some("dark".into()));
    }

    #[test]
    fn stores_bookmarks_history_and_downloads() {
        let store = SqliteStore::in_memory().unwrap();

        assert!(
            store
                .save_bookmark("Example", "https://example.com", "")
                .unwrap()
        );
        assert_eq!(store.list_bookmarks().unwrap().len(), 1);
        assert!(store.remove_bookmark("https://example.com").unwrap());

        store
            .add_history("Example", "https://example.com", "")
            .unwrap();
        assert_eq!(store.list_history().unwrap().len(), 1);
        assert!(store.remove_history("https://example.com").unwrap());

        store
            .upsert_download("https://example.com/file", "/tmp/file", "started", 0)
            .unwrap();
        assert_eq!(store.list_downloads().unwrap()[0].path, "/tmp/file");
        assert!(store.remove_download(None, Some("/tmp/file")).unwrap());
    }

    #[test]
    fn clear_history_range_all_removes_everything() {
        let store = SqliteStore::in_memory().unwrap();
        store.add_history("A", "https://a.com", "").unwrap();
        store.add_history("B", "https://b.com", "").unwrap();
        assert!(store.clear_history_range("all").unwrap());
        assert!(store.list_history().unwrap().is_empty());
    }

    #[test]
    fn clear_history_range_unknown_is_noop() {
        let store = SqliteStore::in_memory().unwrap();
        store.add_history("A", "https://a.com", "").unwrap();
        assert!(!store.clear_history_range("unknown").unwrap());
        assert_eq!(store.list_history().unwrap().len(), 1);
    }

    #[test]
    fn persistent_connections_use_wal_and_busy_timeout() {
        let path = std::env::temp_dir().join(format!(
            "fubuki-store-{}-{}.sqlite3",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let first = SqliteStore::open(&path).unwrap();
            let second = SqliteStore::open(&path).unwrap();
            let journal_mode: String = first
                .conn
                .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .unwrap();
            let busy_timeout: i64 = second
                .conn
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .unwrap();
            let foreign_keys: i64 = first
                .conn
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .unwrap();
            let synchronous: i64 = first
                .conn
                .query_row("PRAGMA synchronous", [], |row| row.get(0))
                .unwrap();
            assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
            assert_eq!(busy_timeout, 5_000);
            assert_eq!(foreign_keys, 1);
            assert_eq!(synchronous, 1); // NORMAL
        }
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }

    #[test]
    fn ephemeral_connections_keep_foreign_keys_and_full_sync() {
        let store = SqliteStore::in_memory().unwrap();
        let foreign_keys: i64 = store
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        let synchronous: i64 = store
            .conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        let busy_timeout: i64 = store
            .conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
        assert_eq!(synchronous, 2); // FULL
        assert_eq!(busy_timeout, 5_000);
    }

    #[test]
    fn concurrent_read_write_connections_do_not_lose_writes() {
        let path = std::env::temp_dir().join(format!(
            "fubuki-store-rw-{}-{}.sqlite3",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        SqliteStore::open(&path).unwrap();
        let writer_path = path.clone();
        let writer = std::thread::spawn(move || {
            let store = SqliteStore::open(writer_path).unwrap();
            for _ in 0..40 {
                store.set_setting("theme", "dark").unwrap();
            }
        });
        let reader_path = path.clone();
        let reader = std::thread::spawn(move || {
            let store = SqliteStore::open(reader_path).unwrap();
            for _ in 0..40 {
                let _ = store.list_bookmarks().unwrap();
            }
        });
        writer.join().unwrap();
        reader.join().unwrap();
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.get_setting("theme").unwrap(), Some("dark".into()));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }

    #[test]
    fn concurrent_migrations_are_serialized_by_busy_timeout() {
        let path = std::env::temp_dir().join(format!(
            "fubuki-store-migration-{}-{}.sqlite3",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    SqliteStore::open(path).unwrap();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.get_setting("theme").unwrap(), None);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }
}
