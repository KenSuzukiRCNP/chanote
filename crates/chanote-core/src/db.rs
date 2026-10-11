use crate::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Embedded migrations, applied in order. `user_version` pragma tracks progress.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_init", include_str!("../migrations/001_init.sql")),
    ("002_online", include_str!("../migrations/002_online.sql")),
    ("003_pack_prefix", include_str!("../migrations/003_pack_prefix.sql")),
    ("004_events", include_str!("../migrations/004_events.sql")),
];

/// Data migrations that need Rust (JSON rewriting etc.), run inside the same
/// transaction right after the SQL of the corresponding version.
fn data_migration(version: i64, tx: &rusqlite::Transaction) -> Result<()> {
    match version {
        3 => crate::migrate::pack_prefix_003(tx),
        _ => Ok(()),
    }
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Open (or create) the database file and bring it up to date.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// In-memory database (tests).
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let db = Db { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }

    fn migrate(&self) -> Result<()> {
        let mut conn = self.conn();
        let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if current >= MIGRATIONS.len() as i64 {
            return Ok(());
        }
        // id rewrites need FK enforcement off; the pragma only takes effect outside a tx
        conn.pragma_update(None, "foreign_keys", false)?;
        let result = (|| -> Result<()> {
            for (i, (_name, sql)) in MIGRATIONS.iter().enumerate() {
                let version = (i + 1) as i64;
                if version <= current {
                    continue;
                }
                let tx = conn.transaction()?;
                tx.execute_batch(sql)?;
                data_migration(version, &tx)?;
                tx.pragma_update(None, "user_version", version)?;
                tx.commit()?;
            }
            Ok(())
        })();
        conn.pragma_update(None, "foreign_keys", true)?;
        result
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.conn().query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn();
        let mut st = conn.prepare("SELECT value FROM setting WHERE key = ?1")?;
        let mut rows = st.query([key])?;
        Ok(match rows.next()? {
            Some(r) => r.get(0)?,
            None => None,
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO setting(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_fresh_db() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len() as i64);
        // idempotent
        db.migrate().unwrap();
        assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len() as i64);
    }

    #[test]
    fn settings_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_setting("ui_lang").unwrap(), None);
        db.set_setting("ui_lang", "ja").unwrap();
        db.set_setting("ui_lang", "en").unwrap();
        assert_eq!(db.get_setting("ui_lang").unwrap().as_deref(), Some("en"));
    }
}
