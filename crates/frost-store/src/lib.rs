use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use frost_protocol::{BookmarkRecord, DownloadRecord, HistoryRecord, PermissionRecord};
use rusqlite::{Connection, ffi, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to prepare database directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("legacy database migration failed: {0}")]
    Migration(String),
    #[error("invalid setting key: {0}")]
    InvalidKey(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

pub trait SettingsRepository {
    fn get_setting(&self, key: &str) -> StoreResult<Option<String>>;
    fn set_setting(&self, key: &str, value: &str) -> StoreResult<()>;
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
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        // SQLite can return BUSY immediately while another first connection
        // changes journal mode, before that connection's busy timeout applies.
        // Reopen with a short bounded backoff so concurrent startup is safe.
        for attempt in 0..16 {
            match Self::open_persistent(path) {
                Ok(store) => return Ok(store),
                Err(error) if is_database_contention(&error) && attempt < 15 => {
                    let delay_ms = 5_u64 << attempt.min(6);
                    std::thread::sleep(Duration::from_millis(delay_ms));
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("bounded SQLite open loop always returns")
    }

    fn open_persistent(path: &Path) -> StoreResult<Self> {
        let store = Self {
            conn: Connection::open(path)?,
        };
        store.configure_connection(ConnectionKind::Persistent)?;
        store.migrate()?;
        store.migrate_legacy_database(path)?;
        Ok(store)
    }

    pub fn in_memory() -> StoreResult<Self> {
        let store = Self {
            conn: Connection::open_in_memory()?,
        };
        store.configure_connection(ConnectionKind::Ephemeral)?;
        store.migrate()?;
        Ok(store)
    }

    fn configure_connection(&self, kind: ConnectionKind) -> StoreResult<()> {
        // The engine worker and native host intentionally use independent
        // connections to the same profile database. WAL keeps normal reads
        // from blocking writes, while the timeout absorbs short write bursts.
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

    fn migrate(&self) -> StoreResult<()> {
        let transaction = self.conn.unchecked_transaction()?;
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
            CREATE TABLE IF NOT EXISTS schema_migrations (
              name TEXT PRIMARY KEY NOT NULL,
              applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Imports the previous host-owned database once, without ever deleting
    /// the original. The migration is transactional and records its marker
    /// only after all copies complete, so a failed run can safely be retried.
    fn migrate_legacy_database(&self, target_path: &Path) -> StoreResult<()> {
        if target_path.file_name().and_then(|name| name.to_str()) != Some("frost-engine.sqlite3") {
            return Ok(());
        }
        let Some(parent) = target_path.parent() else {
            return Ok(());
        };
        let legacy_path = parent.join("fubuki.sqlite3");
        if !legacy_path.is_file() {
            return Ok(());
        }

        let already_migrated: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = 'host-fubuki-sqlite-v1')",
            [],
            |row| row.get(0),
        )?;
        if already_migrated {
            return Ok(());
        }

        let backup_path = parent.join("fubuki.sqlite3.pre-frost-backup");
        if !backup_path.exists() {
            fs::copy(&legacy_path, &backup_path).map_err(|error| {
                StoreError::Migration(format!(
                    "could not back up {} to {}: {error}",
                    legacy_path.display(),
                    backup_path.display()
                ))
            })?;
        }

        self.conn.execute(
            "ATTACH DATABASE ?1 AS legacy",
            [&legacy_path.to_string_lossy()],
        )?;
        let migration = (|| -> StoreResult<()> {
            let tx = self.conn.unchecked_transaction()?;
            copy_legacy_table(
                &tx,
                "settings",
                "INSERT OR REPLACE INTO settings(key, value, updated_at) SELECT key, value, CURRENT_TIMESTAMP FROM legacy.settings",
            )?;
            copy_legacy_table(
                &tx,
                "bookmarks",
                "INSERT OR IGNORE INTO bookmarks(title, url, favicon_url, created_at) SELECT title, url, COALESCE(favicon_url, ''), COALESCE(created_at, strftime('%s','now')) FROM legacy.bookmarks",
            )?;
            copy_legacy_history(&tx)?;
            copy_legacy_downloads(&tx)?;
            copy_legacy_table(
                &tx,
                "logs",
                "INSERT INTO logs(level, message, created_at) SELECT COALESCE(level, 'info'), COALESCE(message, ''), COALESCE(created_at, strftime('%s','now')) FROM legacy.logs",
            )?;
            copy_legacy_table(
                &tx,
                "site_permissions",
                "INSERT OR REPLACE INTO permissions(origin, permission, value, created_at) SELECT origin, permission, value, COALESCE(updated_at, strftime('%s','now')) FROM legacy.site_permissions",
            )?;
            verify_legacy_copy(&tx, "settings", "settings")?;
            verify_legacy_copy(&tx, "bookmarks", "bookmarks")?;
            verify_legacy_copy(&tx, "history", "history")?;
            verify_legacy_copy(&tx, "downloads", "downloads")?;
            verify_legacy_copy(&tx, "logs", "logs")?;
            verify_legacy_copy(&tx, "site_permissions", "permissions")?;
            tx.execute(
                "INSERT INTO schema_migrations(name) VALUES ('host-fubuki-sqlite-v1')",
                [],
            )?;
            tx.commit()?;
            Ok(())
        })();
        let detach = self.conn.execute_batch("DETACH DATABASE legacy");
        match (migration, detach) {
            (Err(migration_error), _) => Err(migration_error),
            (Ok(()), Err(detach_error)) => Err(detach_error.into()),
            (Ok(()), Ok(())) => Ok(()),
        }
    }
}

fn is_database_contention(error: &StoreError) -> bool {
    matches!(
        error,
        StoreError::Sqlite(rusqlite::Error::SqliteFailure(sqlite_error, _))
            if matches!(
                sqlite_error.code,
                ffi::ErrorCode::DatabaseBusy | ffi::ErrorCode::DatabaseLocked
            )
    )
}

#[derive(Clone, Copy)]
enum ConnectionKind {
    Persistent,
    Ephemeral,
}

/// Refuse to mark a legacy migration complete unless every source table that
/// existed has at least as many rows in its destination. This runs inside the
/// same transaction as the copy and marker, so a partial copy rolls back and
/// remains safely retryable from the untouched backup/source database.
fn verify_legacy_copy(
    tx: &rusqlite::Transaction<'_>,
    legacy_table: &str,
    target_table: &str,
) -> StoreResult<()> {
    if !legacy_table_exists(tx, legacy_table)? {
        return Ok(());
    }
    let legacy_count: i64 = tx.query_row(
        &format!("SELECT COUNT(*) FROM legacy.{legacy_table}"),
        [],
        |row| row.get(0),
    )?;
    let target_count: i64 =
        tx.query_row(&format!("SELECT COUNT(*) FROM {target_table}"), [], |row| {
            row.get(0)
        })?;
    if target_count < legacy_count {
        return Err(StoreError::Migration(format!(
            "legacy table {legacy_table} copied only {target_count} of {legacy_count} rows into {target_table}"
        )));
    }
    Ok(())
}

fn legacy_table_exists(tx: &rusqlite::Transaction<'_>, table: &str) -> StoreResult<bool> {
    Ok(tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM legacy.sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )?)
}

fn legacy_column_exists(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    column: &str,
) -> StoreResult<bool> {
    if !legacy_table_exists(tx, table)? {
        return Ok(false);
    }
    let mut statement = tx.prepare(&format!("PRAGMA legacy.table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in columns {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn copy_legacy_table(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    statement: &str,
) -> StoreResult<()> {
    if legacy_table_exists(tx, table)? {
        tx.execute_batch(statement)?;
    }
    Ok(())
}

fn copy_legacy_history(tx: &rusqlite::Transaction<'_>) -> StoreResult<()> {
    if !legacy_table_exists(tx, "history")? {
        return Ok(());
    }
    let favicon = if legacy_column_exists(tx, "history", "favicon_url")? {
        "COALESCE(favicon_url, '')"
    } else {
        "''"
    };
    tx.execute_batch(&format!(
        "INSERT INTO history(title, url, favicon_url, created_at) SELECT title, url, {favicon}, COALESCE(created_at, strftime('%s','now')) FROM legacy.history"
    ))?;
    Ok(())
}

fn copy_legacy_downloads(tx: &rusqlite::Transaction<'_>) -> StoreResult<()> {
    if !legacy_table_exists(tx, "downloads")? {
        return Ok(());
    }
    let created = if legacy_column_exists(tx, "downloads", "created_at")? {
        "COALESCE(created_at, strftime('%s','now'))"
    } else {
        "strftime('%s','now')"
    };
    let updated = if legacy_column_exists(tx, "downloads", "updated_at")? {
        format!("COALESCE(updated_at, {created})")
    } else {
        created.to_owned()
    };
    tx.execute_batch(&format!(
        "INSERT INTO downloads(url, path, state, percent, created_at, updated_at) SELECT COALESCE(url, ''), COALESCE(path, ''), COALESCE(state, 'unknown'), COALESCE(percent, 0), {created}, {updated} FROM legacy.downloads"
    ))?;
    Ok(())
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
    use std::fs;

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
    fn persistent_connections_enable_wal_and_wait_for_contention() {
        let path = unique_database_path("connection-options");
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
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
        assert_eq!(busy_timeout, 5_000);
        assert_eq!(foreign_keys, 1);

        drop((first, second));
        remove_database(&path);
    }

    #[test]
    fn concurrent_connections_preserve_reads_and_writes() {
        let path = unique_database_path("concurrent-access");
        SqliteStore::open(&path).unwrap();

        let writer_path = path.clone();
        let writer = std::thread::spawn(move || {
            let store = SqliteStore::open(writer_path).unwrap();
            for index in 0..50 {
                store
                    .set_setting("concurrent-write", &index.to_string())
                    .unwrap();
            }
        });
        let reader_path = path.clone();
        let reader = std::thread::spawn(move || {
            let store = SqliteStore::open(reader_path).unwrap();
            for _ in 0..50 {
                store.list_history().unwrap();
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
        assert_eq!(
            SqliteStore::open(&path)
                .unwrap()
                .get_setting("concurrent-write")
                .unwrap(),
            Some("49".into())
        );
        remove_database(&path);
    }

    #[test]
    fn concurrent_first_open_serializes_migrations() {
        let path = unique_database_path("concurrent-migration");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    SqliteStore::open(path).unwrap();
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }

        assert!(SqliteStore::open(&path).is_ok());
        remove_database(&path);
    }

    #[test]
    fn migrates_legacy_database_transactionally_without_removing_source() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile = std::env::temp_dir().join(format!(
            "frost-store-migration-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&profile).unwrap();
        let legacy = profile.join("fubuki.sqlite3");
        let legacy_conn = Connection::open(&legacy).unwrap();
        legacy_conn
            .execute_batch(
                "
                CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                CREATE TABLE history(id INTEGER PRIMARY KEY, title TEXT NOT NULL, url TEXT NOT NULL, created_at TEXT NOT NULL);
                INSERT INTO settings(key, value) VALUES ('theme', 'dark');
                INSERT INTO history(title, url, created_at) VALUES ('Example', 'https://example.com', '1');
                ",
            )
            .unwrap();
        drop(legacy_conn);

        let target = profile.join("frost-engine.sqlite3");
        let store = SqliteStore::open(&target).unwrap();
        assert_eq!(store.get_setting("theme").unwrap(), Some("dark".into()));
        assert_eq!(store.list_history().unwrap()[0].favicon_url, "");
        drop(store);

        assert!(legacy.is_file());
        assert!(profile.join("fubuki.sqlite3.pre-frost-backup").is_file());
        let reopened = SqliteStore::open(&target).unwrap();
        assert_eq!(reopened.list_history().unwrap().len(), 1);
        drop(reopened);
        fs::remove_dir_all(profile).unwrap();
    }

    fn unique_database_path(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "fubuki-store-{label}-{}-{suffix}.sqlite3",
            std::process::id()
        ))
    }

    fn remove_database(path: &Path) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }
}
