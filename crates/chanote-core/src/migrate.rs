//! Data migrations that are easier in Rust than in SQL.

use crate::Result;
use rusqlite::{params, Transaction};
use std::collections::HashMap;

/// Kinds whose pack rows are scoped by pack id.
pub const SCOPED_KINDS: &[&str] = &["temae", "temae-cat", "tana"];

/// Compute the pack-scoped id for a row: `temae.karamono` (origin `pack:urasenke`) →
/// `temae.urasenke.karamono`. Ids that already carry the pack segment are left alone.
pub fn scoped_id(kind: &str, id: &str, pack: &str) -> Option<String> {
    let rest = id.strip_prefix(&format!("{kind}."))?;
    if rest.starts_with(&format!("{pack}.")) {
        return None;
    }
    Some(format!("{kind}.{pack}.{rest}"))
}

/// Rewrite one id through the map (identity when absent).
fn map_id(m: &HashMap<String, String>, id: &str) -> String {
    m.get(id).cloned().unwrap_or_else(|| id.to_string())
}

/// Apply an id map to a vocab `meta` JSON string (alias_of, defaults.tana, modifiers.tana).
pub fn rewrite_meta(m: &HashMap<String, String>, meta: &str) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(meta).ok()?;
    let mut changed = false;
    if let Some(a) = v.get("alias_of").and_then(|x| x.as_str()).map(String::from) {
        let n = map_id(m, &a);
        if n != a { v["alias_of"] = serde_json::Value::String(n); changed = true; }
    }
    for key in ["defaults", "modifiers"] {
        if let Some(t) = v.get(key).and_then(|x| x.get("tana")).and_then(|x| x.as_str()).map(String::from) {
            let n = map_id(m, &t);
            if n != t { v[key]["tana"] = serde_json::Value::String(n); changed = true; }
        }
    }
    if let Some(p) = v.get("paired_with").and_then(|x| x.as_str()).map(String::from) {
        let n = map_id(m, &p);
        if n != p { v["paired_with"] = serde_json::Value::String(n); changed = true; }
    }
    changed.then(|| v.to_string())
}

/// Build the old→new id map for all pack rows of scoped kinds present in `vocab`.
pub fn pack_prefix_map(tx: &Transaction) -> Result<HashMap<String, String>> {
    let mut st = tx.prepare("SELECT id, kind, origin FROM vocab WHERE origin LIKE 'pack:%'")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
    let mut m = HashMap::new();
    for row in rows {
        let (id, kind, origin) = row?;
        if !SCOPED_KINDS.contains(&kind.as_str()) { continue; }
        let pack = origin.trim_start_matches("pack:");
        if let Some(new) = scoped_id(&kind, &id, pack) {
            m.insert(id, new);
        }
    }
    Ok(m)
}

/// Rewrite every reference to the ids in `m`, in vocab and in records.
pub fn apply_id_map(tx: &Transaction, m: &HashMap<String, String>) -> Result<()> {
    if m.is_empty() {
        return Ok(());
    }
    for (old, new) in m {
        tx.execute("UPDATE vocab SET id = ?2 WHERE id = ?1", params![old, new])?;
        tx.execute("UPDATE vocab SET parent_id = ?2 WHERE parent_id = ?1", params![old, new])?;
        tx.execute("UPDATE temae SET temae_id = ?2 WHERE temae_id = ?1", params![old, new])?;
        tx.execute("UPDATE temae SET temae_alias_id = ?2 WHERE temae_alias_id = ?1", params![old, new])?;
        tx.execute("UPDATE temae SET tana_id = ?2 WHERE tana_id = ?1", params![old, new])?;
        tx.execute("UPDATE temae_tool SET tool_id = ?2 WHERE tool_id = ?1", params![old, new])?;
    }
    // JSON references inside meta (any row, pack or user)
    let rows: Vec<(String, String)> = {
        let mut st = tx.prepare("SELECT id, meta FROM vocab WHERE meta IS NOT NULL")?;
        let v = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        v
    };
    for (id, meta) in rows {
        if let Some(new_meta) = rewrite_meta(m, &meta) {
            tx.execute("UPDATE vocab SET meta = ?2 WHERE id = ?1", params![id, new_meta])?;
        }
    }
    Ok(())
}

/// Migration 003: scope pack ids by pack.
pub fn pack_prefix_003(tx: &Transaction) -> Result<()> {
    let m = pack_prefix_map(tx)?;
    apply_id_map(tx, &m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_ids() {
        assert_eq!(scoped_id("temae", "temae.karamono", "urasenke").as_deref(), Some("temae.urasenke.karamono"));
        assert_eq!(scoped_id("temae", "temae.urasenke.karamono", "urasenke"), None);
        assert_eq!(scoped_id("tana", "tana.daisu.shin", "urasenke").as_deref(), Some("tana.urasenke.daisu.shin"));
        assert_eq!(scoped_id("temae", "place.x", "urasenke"), None);
    }

    #[test]
    fn meta_rewrite() {
        let mut m = HashMap::new();
        m.insert("temae.kagetsu".to_string(), "temae.urasenke.kagetsu".to_string());
        m.insert("tana.daisu.shin".to_string(), "tana.urasenke.daisu.shin".to_string());
        let out = rewrite_meta(&m, r#"{"alias_of":"temae.kagetsu","modifiers":{"tana":"tana.daisu.shin","tea":"koicha"}}"#).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["alias_of"], "temae.urasenke.kagetsu");
        assert_eq!(v["modifiers"]["tana"], "tana.urasenke.daisu.shin");
        assert_eq!(v["modifiers"]["tea"], "koicha");
        assert!(rewrite_meta(&m, r#"{"note":"x"}"#).is_none());
    }

    /// Simulate a v0.2 database (schema 2, unscoped ids) and open it with the current code.
    #[test]
    fn upgrades_v02_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(include_str!("../migrations/001_init.sql")).unwrap();
            conn.execute_batch(include_str!("../migrations/002_online.sql")).unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
            conn.execute_batch(r#"
                INSERT INTO vocab (id, kind, ja, origin, meta) VALUES
                  ('temae-cat.kagetsu', 'temae-cat', '花月之式', 'pack:urasenke', NULL),
                  ('temae.kagetsu', 'temae', '花月之式', 'pack:urasenke', '{"category":"kagetsu","defaults":{"tea":"usucha","tana":"-"}}'),
                  ('temae.kagetsu.koicha', 'temae', '濃茶付花月', 'pack:urasenke', '{"alias_of":"temae.kagetsu","modifiers":{"tea":"koicha"}}'),
                  ('tana.daisu', 'tana', '台子', 'pack:urasenke', NULL),
                  ('tana.daisu.shin', 'tana', '真台子', 'pack:urasenke', NULL),
                  ('temae.shin-no-gyo', 'temae', '真之行台子', 'pack:urasenke', '{"defaults":{"tea":"koicha","tana":"tana.daisu.shin"}}'),
                  ('temae.nagaita-koicha-abc', 'temae', '長板濃茶', 'user', '{"alias_of":"temae.kagetsu","modifiers":{"tana":"tana.daisu.shin"}}'),
                  ('person.self', 'person', '自分', 'user', '{"is_self":true}');
                UPDATE vocab SET parent_id = 'temae-cat.kagetsu' WHERE id IN ('temae.kagetsu','temae.kagetsu.koicha');
                UPDATE vocab SET parent_id = 'tana.daisu' WHERE id = 'tana.daisu.shin';
                INSERT INTO session (id, date, kind) VALUES (1, '2026-01-01', 'honban'), (2, '2026-01-02', 'keiko');
                INSERT INTO session_person (session_id, person_id, role) VALUES (2, 'person.self', 'teacher');
                INSERT INTO temae (session_id, ord, temae_id, temae_alias_id, tana_id, host_id, my_role)
                  VALUES (1, 0, 'temae.kagetsu', 'temae.kagetsu.koicha', 'tana.daisu.shin', 'person.self', 'teishu');
            "#).unwrap();
        }
        let db = crate::Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 4);
        // 004: kind → form / practice
        let s1 = db.session_get(1).unwrap().unwrap();
        assert_eq!((s1.form.as_str(), s1.practice), ("?", false));
        let s2 = db.session_get(2).unwrap().unwrap();
        assert_eq!((s2.form.as_str(), s2.practice), ("temae", true));
        assert_eq!(s2.persons[0].role, "teacher");
        assert!(db.vocab_get("temae.kagetsu").unwrap().is_none());
        let k = db.vocab_get("temae.urasenke.kagetsu").unwrap().unwrap();
        assert_eq!(k.parent_id.as_deref(), Some("temae-cat.urasenke.kagetsu"));
        let shin = db.vocab_get("tana.urasenke.daisu.shin").unwrap().unwrap();
        assert_eq!(shin.parent_id.as_deref(), Some("tana.urasenke.daisu"));
        // user alias now points at the scoped ids
        let user = db.vocab_get("temae.nagaita-koicha-abc").unwrap().unwrap();
        let meta = user.meta.unwrap();
        assert_eq!(meta["alias_of"], "temae.urasenke.kagetsu");
        assert_eq!(meta["modifiers"]["tana"], "tana.urasenke.daisu.shin");
        // records
        let s = db.session_get(1).unwrap().unwrap();
        assert_eq!(s.temae[0].temae_id.as_deref(), Some("temae.urasenke.kagetsu"));
        assert_eq!(s.temae[0].temae_alias_id.as_deref(), Some("temae.urasenke.kagetsu.koicha"));
        assert_eq!(s.temae[0].tana_id.as_deref(), Some("tana.urasenke.daisu.shin"));
        // resolving still works and installing the current pack upgrades in place
        let r = db.resolve_temae("temae.urasenke.kagetsu.koicha").unwrap();
        assert_eq!(r.axes.tea.as_deref(), Some("koicha"));
        db.pack_install(&crate::pack::Pack::from_yaml(crate::pack::URASENKE_YAML).unwrap()).unwrap();
        assert!(db.vocab_get("temae.urasenke.homa").unwrap().is_some());
        // no duplicate unscoped rows were created
        assert!(db.vocab_get("temae.kagetsu").unwrap().is_none());
    }
}
