use std::path::Path;

use rusqlite::{params, Connection};
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
            ",
        )?;
        Ok(())
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
}
