use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use frost_protocol::{BookmarkRecord, DownloadRecord, HistoryRecord, PermissionRecord};
use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
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

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let store = Self {
            conn: Connection::open(path)?,
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> StoreResult<Self> {
        let store = Self {
            conn: Connection::open_in_memory()?,
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> StoreResult<()> {
        self.conn.execute_batch(
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
            ",
        )?;
        Ok(())
    }
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
}
