use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use frost_protocol::{BookmarkRecord, DownloadRecord, HistoryRecord, PermissionRecord};
use rusqlite::{Connection, params};
use thiserror::Error;

pub const VALID_SETTING_KEYS: &[&str] = &[
    "homepage",
    "downloadDirectory",
    "searchEngine",
    "startupBehavior",
    "customSearchUrl",
    "theme",
    "appearance",
    "toolbarDensity",
    "sidebarVisible",
    "sidebarWidth",
    "defaultBookmarkDisplay",
    "openBookmarkIn",
    "showBookmarkFavicons",
    "newTabPage",
    "homeUrl",
    "askBeforeDownload",
    "language",
    "defaultZoomLevel",
    "closeWindowWithLastTab",
    "privateSearchEngine",
    "newTabBackgroundMode",
    "newTabBackgroundColor",
    "newTabBackgroundUrl",
];

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid setting key: {0}")]
    InvalidKey(String),
    #[error("database schema version {found} is newer than supported version {supported}")]
    UnsupportedSchemaVersion { found: i64, supported: i64 },
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
        let mut store = Self {
            conn: Connection::open(path)?,
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> StoreResult<Self> {
        let mut store = Self {
            conn: Connection::open_in_memory()?,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> StoreResult<()> {
        migrations::migrate(&mut self.conn)
    }
}

mod migrations {
    use super::{Connection, StoreError, StoreResult};

    pub(super) const LATEST_SCHEMA_VERSION: i64 = 2;

    struct Migration {
        version: i64,
        sql: &'static str,
    }

    const MIGRATIONS: &[Migration] = &[
        Migration {
            version: 1,
            sql: "
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
        },
        Migration {
            version: 2,
            sql: "
            CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
            CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at);
            CREATE INDEX IF NOT EXISTS idx_downloads_url_path ON downloads(url, path);
            CREATE INDEX IF NOT EXISTS idx_downloads_path ON downloads(path);
            ",
        },
    ];

    pub(super) fn migrate(conn: &mut Connection) -> StoreResult<()> {
        let current_version = schema_version(conn)?;
        if current_version > LATEST_SCHEMA_VERSION {
            return Err(StoreError::UnsupportedSchemaVersion {
                found: current_version,
                supported: LATEST_SCHEMA_VERSION,
            });
        }

        for migration in MIGRATIONS
            .iter()
            .filter(|migration| migration.version > current_version)
        {
            apply(conn, migration.version, migration.sql)?;
        }
        Ok(())
    }

    fn schema_version(conn: &Connection) -> rusqlite::Result<i64> {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
    }

    fn apply(conn: &mut Connection, version: i64, sql: &str) -> StoreResult<()> {
        let transaction = conn.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn apply_for_test(
        conn: &mut Connection,
        version: i64,
        sql: &str,
    ) -> StoreResult<()> {
        apply(conn, version, sql)
    }

    #[cfg(test)]
    pub(super) fn apply_up_to_for_test(
        conn: &mut Connection,
        target_version: i64,
    ) -> StoreResult<()> {
        for migration in MIGRATIONS
            .iter()
            .filter(|migration| migration.version <= target_version)
        {
            apply(conn, migration.version, migration.sql)?;
        }
        Ok(())
    }
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
        if !VALID_SETTING_KEYS.contains(&key) {
            return Err(StoreError::InvalidKey(key.to_owned()));
        }
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
        let rows = stmt.query_map(params![limit as i64], |row| {
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

    fn schema_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    fn schema_objects(conn: &Connection, object_type: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = ?1 ORDER BY name")
            .unwrap();
        stmt.query_map(params![object_type], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn empty_database_migrates_to_latest_schema() {
        let store = SqliteStore::in_memory().unwrap();

        assert_eq!(
            schema_version(&store.conn),
            migrations::LATEST_SCHEMA_VERSION
        );
        let tables = schema_objects(&store.conn, "table");
        for table in [
            "settings",
            "bookmarks",
            "history",
            "downloads",
            "permissions",
            "logs",
            "session",
        ] {
            assert!(tables.iter().any(|name| name == table), "missing {table}");
        }
    }

    #[test]
    fn version_one_fixture_migrates_and_preserves_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_up_to_for_test(&mut conn, 1).unwrap();
        conn.execute(
            "INSERT INTO history(title, url, created_at) VALUES ('Example', 'https://example.com', '1')",
            [],
        )
        .unwrap();

        migrations::migrate(&mut conn).unwrap();

        assert_eq!(schema_version(&conn), migrations::LATEST_SCHEMA_VERSION);
        assert_eq!(
            conn.query_row("SELECT url FROM history", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "https://example.com"
        );
    }

    #[test]
    fn failed_migration_rolls_back_data_and_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        let previous_version = schema_version(&conn);

        let result = migrations::apply_for_test(
            &mut conn,
            previous_version + 1,
            "
            INSERT INTO settings(key, value) VALUES ('theme', 'dark');
            INSERT INTO missing_table(value) VALUES ('fail');
            ",
        );

        assert!(result.is_err());
        assert_eq!(schema_version(&conn), previous_version);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn rerunning_migrations_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::migrate(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO bookmarks(title, url, created_at) VALUES ('Example', 'https://example.com', '1')",
            [],
        )
        .unwrap();
        let objects_before = schema_objects(&conn, "index");

        migrations::migrate(&mut conn).unwrap();

        assert_eq!(schema_version(&conn), migrations::LATEST_SCHEMA_VERSION);
        assert_eq!(schema_objects(&conn, "index"), objects_before);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn unknown_setting_key_is_rejected_without_writing() {
        let store = SqliteStore::in_memory().unwrap();

        let result = store.set_setting("unknownSetting", "value");

        assert!(matches!(
            result,
            Err(StoreError::InvalidKey(key)) if key == "unknownSetting"
        ));
        assert_eq!(store.get_setting("unknownSetting").unwrap(), None);
    }

    #[test]
    fn every_allowlisted_setting_key_can_be_written() {
        let store = SqliteStore::in_memory().unwrap();

        for key in VALID_SETTING_KEYS {
            store.set_setting(key, "value").unwrap();
        }

        let count = store
            .conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap();
        assert_eq!(count, VALID_SETTING_KEYS.len() as i64);
    }

    #[test]
    fn latest_schema_has_history_and_download_indexes() {
        let store = SqliteStore::in_memory().unwrap();
        let indexes = schema_objects(&store.conn, "index");

        for index in [
            "idx_history_url",
            "idx_history_created_at",
            "idx_downloads_url_path",
            "idx_downloads_path",
        ] {
            assert!(indexes.iter().any(|name| name == index), "missing {index}");
        }
    }

    #[test]
    fn newer_schema_version_is_rejected_without_changes() {
        let mut conn = Connection::open_in_memory().unwrap();
        let future_version = migrations::LATEST_SCHEMA_VERSION + 1;
        conn.pragma_update(None, "user_version", future_version)
            .unwrap();

        let result = migrations::migrate(&mut conn);

        assert!(matches!(
            result,
            Err(StoreError::UnsupportedSchemaVersion { found, supported })
                if found == future_version && supported == migrations::LATEST_SCHEMA_VERSION
        ));
        assert_eq!(schema_version(&conn), future_version);
        assert!(schema_objects(&conn, "table").is_empty());
    }

    #[test]
    fn unversioned_legacy_schema_migrates_without_losing_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE settings (
              key TEXT PRIMARY KEY NOT NULL,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO settings(key, value) VALUES ('theme', 'dark');
            ",
        )
        .unwrap();

        migrations::migrate(&mut conn).unwrap();

        assert_eq!(schema_version(&conn), migrations::LATEST_SCHEMA_VERSION);
        assert_eq!(
            conn.query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "dark"
        );
    }

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
}
