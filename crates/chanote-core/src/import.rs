//! Re-import of a chanote JSON export. Vocabulary rows that do not exist yet are added
//! (existing rows are never overwritten); sessions are appended, skipping ones that look
//! like exact duplicates of a session already in the database.

use crate::export::ExportDoc;
use crate::model::Session;
use crate::{Db, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportReport {
    pub vocab_added: usize,
    pub vocab_skipped: usize,
    pub sessions_added: usize,
    pub sessions_skipped: usize,
}

/// Fingerprint used for duplicate detection: date, place, and the ordered temae ids.
fn fingerprint(s: &Session) -> String {
    let temae: Vec<String> = s
        .temae
        .iter()
        .map(|t| {
            format!(
                "{}|{}|{}|{}",
                t.temae_id.clone().unwrap_or_default(),
                t.temae_free.clone().unwrap_or_default(),
                t.host_id.clone().unwrap_or_default(),
                t.my_role.clone().unwrap_or_default()
            )
        })
        .collect();
    format!(
        "{}#{}#{}#{}",
        s.date.clone().unwrap_or_default(),
        s.date_prec,
        s.place_id.clone().unwrap_or_default(),
        temae.join(";")
    )
}

impl Db {
    pub fn import_json(&self, json: &str) -> Result<ImportReport> {
        let doc: ExportDoc = serde_json::from_str(json)?;
        if doc.app != "chanote" {
            return Err(Error::Other(format!("not a chanote export (app = {})", doc.app)));
        }
        if doc.schema_version > crate::export::SCHEMA_VERSION {
            return Err(Error::Other(format!(
                "export schema {} is newer than this app supports ({})",
                doc.schema_version,
                crate::export::SCHEMA_VERSION
            )));
        }
        let mut rep = ImportReport::default();

        // vocab: parents first so foreign keys hold — the export is already emitted in
        // dependency-friendly order (school, categories, temae, ...), but a user row can
        // reference a parent later in the list, so do two passes.
        let mut pending = doc.vocab.clone();
        for _ in 0..3 {
            let mut retry = Vec::new();
            for v in pending {
                if self.vocab_get(&v.id)?.is_some() {
                    rep.vocab_skipped += 1;
                    continue;
                }
                match self.vocab_upsert(&v) {
                    Ok(()) => rep.vocab_added += 1,
                    Err(_) => retry.push(v),
                }
            }
            pending = retry;
            if pending.is_empty() {
                break;
            }
        }
        if !pending.is_empty() {
            return Err(Error::Other(format!(
                "{} vocabulary rows could not be imported (missing parents?)",
                pending.len()
            )));
        }

        // existing fingerprints
        let existing: HashSet<String> = {
            let ids: Vec<i64> = {
                let conn = self.conn();
                let mut st = conn.prepare("SELECT id FROM session")?;
                let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
                v
            };
            let mut set = HashSet::new();
            for id in ids {
                if let Some(s) = self.session_get(id)? {
                    set.insert(fingerprint(&s));
                }
            }
            set
        };

        for mut s in doc.sessions {
            let fp = fingerprint(&s);
            if existing.contains(&fp) {
                rep.sessions_skipped += 1;
                continue;
            }
            s.id = None;
            for t in s.temae.iter_mut() {
                t.id = None;
            }
            self.session_save(&s)?;
            rep.sessions_added += 1;
        }
        Ok(rep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Temae;
    use crate::pack::{Pack, URASENKE_YAML};

    #[test]
    fn roundtrip_export_import_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        let me = db.self_person_id().unwrap();
        db.session_save(&Session {
            date: Some("2026-05-10".into()),
            temae: vec![Temae { temae_id: Some("temae.karamono".into()), host_id: Some(me), my_role: Some("teishu".into()), ..Default::default() }],
            ..Default::default()
        })
        .unwrap();
        let json = db.export_json().unwrap();

        // import into the same db → everything skipped
        let rep = db.import_json(&json).unwrap();
        assert_eq!(rep.sessions_added, 0);
        assert_eq!(rep.sessions_skipped, 1);
        assert_eq!(rep.vocab_added, 0);

        // import into a fresh db (no pack) → vocab + session added
        let fresh = Db::open_in_memory().unwrap();
        let rep = fresh.import_json(&json).unwrap();
        assert_eq!(rep.sessions_added, 1);
        assert!(rep.vocab_added > 50);
        assert_eq!(fresh.session_list(10, 0).unwrap().len(), 1);

        assert!(db.import_json("{\"app\":\"other\"}").is_err());
    }
}
