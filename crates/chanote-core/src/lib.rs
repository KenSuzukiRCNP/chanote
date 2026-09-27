//! chanote-core: everything that touches the database, independent of Tauri.
//!
//! Layout
//! - `db`     : opening the SQLite file, running migrations
//! - `pack`   : school packs (packs/*.yaml) → `vocab` rows
//! - `vocab`  : vocabulary queries
//! - `record` : sessions and temae (CRUD)
//! - `stats`  : aggregations for the statistics screens
//! - `ranking`: recency-weighted usage scores for pick lists
//! - `export` : JSON / CSV export
//! - `import` : re-import of chanote JSON
//! - `attachment` : photos attached to sessions (files in the data dir, thumbnails)
//! - `backup` : zip of the database + attachments, and restore
//! - `model`  : serde types shared with the frontend

pub mod attachment;
pub mod backup;
pub mod db;
pub mod export;
pub mod import;
pub mod migrate;
pub mod model;
pub mod pack;
pub mod ranking;
pub mod record;
pub mod stats;
pub mod vocab;

pub use db::Db;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("pack error: {0}")]
    Pack(String),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("still used by {0} record(s)")]
    InUse(i64),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Reserved vocab ids usable in any axis column.
pub const UNKNOWN: &str = "?";
pub const NONE: &str = "-";
