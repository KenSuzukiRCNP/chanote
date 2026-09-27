//! Re-import of a chanote JSON export. Vocabulary rows that do not exist yet are added
//! (existing rows are never overwritten); sessions are appended, skipping ones that look
//! like exact duplicates of a session already in the database.

use crate::export::ExportDoc;
use crate::model::Session;
use crate::{Db, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportReport {
    pub vocab_added: usize,
    pub vocab_skipped: usize,
    pub sessions_added: usize,
    pub sessions_skipped: usize,
}

/// Bring an older export up to the current schema in memory.
/// schema 1 → 2: pack ids become pack-scoped (see migrate.rs).
pub fn upgrade_doc(mut doc: ExportDoc) -> ExportDoc {
    if doc.schema_version >= 2 {
        return doc;
    }
    let mut m = std::collections::HashMap::new();
    for v in &doc.vocab {
        if let Some(pack) = v.origin.strip_prefix("pack:") {
            if crate::migrate::SCOPED_KINDS.contains(&v.kind.as_str()) {
                if let Some(new) = crate::migrate::scoped_id(&v.kind, &v.id, pack) {
                    m.insert(v.id.clone(), new);
                }
            }
        }
    }
    let map = |id: &Option<String>| id.as_ref().map(|i| m.get(i).cloned().unwrap_or_else(|| i.clone()));
    for v in doc.vocab.iter_mut() {
        if let Some(n) = m.get(&v.id) { v.id = n.clone(); }
        v.parent_id = map(&v.parent_id);
        if let Some(meta) = &v.meta {
            if let Some(new) = crate::migrate::rewrite_meta(&m, &meta.to_string()) {
                v.meta = serde_json::from_str(&new).ok();
            }
        }
    }
    for s in doc.sessions.iter_mut() {
        for t in s.temae.iter_mut() {
            t.temae_id = map(&t.temae_id);
            t.temae_alias_id = map(&t.temae_alias_id);
            t.tana_id = map(&t.tana_id);
        }
    }
    doc.schema_version = 2;
    doc
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

/// Options for importing someone else's export as one's own diary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Person id *in the file* to treat as myself. None = the file's own "self".
    pub self_as: Option<String>,
    /// Name to give the file's original author when `self_as` is set (they become an
    /// ordinary person in my diary).
    pub original_self_name: Option<String>,
    /// Session indices (in file order) to import. None = all sessions that are not
    /// duplicates of an existing record.
    pub include: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreviewPerson {
    pub id: String,
    pub name: String,
    pub is_self: bool,
    /// how often this person appears in the file (host / participant / teacher)
    pub n: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreviewRow {
    pub index: usize,
    pub date: Option<String>,
    pub date_prec: String,
    pub place: String,
    pub kind: String,
    /// "真之行台子 / 濃茶付花月" — temae names in the file's own vocabulary
    pub summary: String,
    pub n_temae: usize,
    /// existing session id when the fingerprint matches
    pub duplicate_of: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportPreview {
    pub persons: Vec<PreviewPerson>,
    pub rows: Vec<PreviewRow>,
    pub vocab_new: usize,
    pub schema_version: u32,
}

fn parse(json: &str) -> Result<ExportDoc> {
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
    Ok(upgrade_doc(doc))
}

/// Rewrite the document so that `self_as` (a person in the file) becomes my "self" and the
/// file's original self becomes an ordinary person named `original_name`.
pub fn swap_self(doc: &mut ExportDoc, self_as: &str, my_self_id: &str, original_name: &str) {
    let file_self = doc
        .vocab
        .iter()
        .find(|v| v.kind == "person" && v.meta.as_ref().and_then(|m| m.get("is_self")).and_then(|x| x.as_bool()) == Some(true))
        .map(|v| v.id.clone());
    let Some(file_self) = file_self else { return };
    if file_self == self_as {
        return;
    }
    let slug: String = original_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let new_author_id = format!("person.{}-{}", if slug.is_empty() { "author".into() } else { slug }, &file_self.len().to_string());
    let map = |id: &Option<String>| -> Option<String> {
        match id.as_deref() {
            Some(x) if x == self_as => Some(my_self_id.to_string()),
            Some(x) if x == file_self => Some(new_author_id.clone()),
            other => other.map(String::from),
        }
    };

    // vocab: the chosen person disappears (it is me); the author becomes a normal person
    doc.vocab.retain(|v| v.id != self_as);
    for v in doc.vocab.iter_mut() {
        if v.id == file_self {
            v.id = new_author_id.clone();
            v.ja = original_name.to_string();
            v.romaji = None;
            v.meta = None;
        }
    }

    for s in doc.sessions.iter_mut() {
        for p in s.persons.iter_mut() {
            p.person_id = map(&p.person_id);
        }
        for t in s.temae.iter_mut() {
            let host_was_author = t.host_id.as_deref() == Some(file_self.as_str());
            let host_is_me = t.host_id.as_deref() == Some(self_as);
            // the author's own participation becomes a participant row
            let author_role = t.my_role.clone();
            let author_seat = t.my_seat;
            let author_note = t.my_role_note.clone();
            // my new role: from host or from the participants list
            let mine = t.persons.iter().position(|p| p.person_id.as_deref() == Some(self_as));
            let (new_role, new_seat, new_note) = if host_is_me {
                (Some("teishu".to_string()), None, None)
            } else if let Some(i) = mine {
                let p = t.persons.remove(i);
                let role = if p.role == "kyaku" && p.seat == Some(1) { "shokyaku".to_string() } else { p.role };
                (Some(role), p.seat, p.role_note)
            } else {
                (None, None, None)
            };
            if !host_was_author {
                if let Some(r) = author_role.as_deref().filter(|r| *r != "teishu" && *r != "-") {
                    let (role, seat) = match r {
                        "shokyaku" => ("kyaku".to_string(), Some(1)),
                        other => (other.to_string(), author_seat),
                    };
                    t.persons.push(crate::model::TemaePerson {
                        person_id: Some(new_author_id.clone()),
                        name_free: None,
                        role,
                        seat,
                        is_last: false,
                        role_note: author_note,
                    });
                }
            }
            t.my_role = new_role;
            t.my_seat = new_seat;
            t.my_role_note = new_note;
            t.host_id = map(&t.host_id);
            t.teacher_id = map(&t.teacher_id);
            for p in t.persons.iter_mut() {
                p.person_id = map(&p.person_id);
            }
        }
    }
}

impl Db {
    fn existing_fingerprints(&self) -> Result<std::collections::HashMap<String, i64>> {
        let ids: Vec<i64> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT id FROM session")?;
            let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        let mut m = std::collections::HashMap::new();
        for id in ids {
            if let Some(s) = self.session_get(id)? {
                m.insert(fingerprint(&s), id);
            }
        }
        Ok(m)
    }

    fn prepared_doc(&self, json: &str, opts: &ImportOptions) -> Result<ExportDoc> {
        let mut doc = parse(json)?;
        if let Some(x) = &opts.self_as {
            let me = self.self_person_id()?;
            swap_self(&mut doc, x, &me, opts.original_self_name.as_deref().unwrap_or("(author)"));
        }
        Ok(doc)
    }

    /// Inspect a file before importing: people in it, one row per session with a
    /// duplicate flag, and how many vocabulary entries would be added.
    pub fn import_preview(&self, json: &str, opts: &ImportOptions) -> Result<ImportPreview> {
        let raw = parse(json)?;
        let mut persons: Vec<PreviewPerson> = raw
            .vocab
            .iter()
            .filter(|v| v.kind == "person")
            .map(|v| PreviewPerson {
                id: v.id.clone(),
                name: v.ja.clone(),
                is_self: v.meta.as_ref().and_then(|m| m.get("is_self")).and_then(|x| x.as_bool()) == Some(true),
                n: 0,
            })
            .collect();
        let self_id = persons.iter().find(|p| p.is_self).map(|p| p.id.clone());
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut bump = |id: &Option<String>| {
            if let Some(i) = id { *counts.entry(i.clone()).or_insert(0) += 1; }
        };
        for s in &raw.sessions {
            for p in &s.persons { bump(&p.person_id); }
            for t in &s.temae {
                bump(&t.host_id);
                bump(&t.teacher_id);
                for p in &t.persons { bump(&p.person_id); }
                if t.my_role.as_deref().map(|r| r != "-").unwrap_or(false) { bump(&self_id); }
            }
        }
        for p in persons.iter_mut() { p.n = counts.get(&p.id).copied().unwrap_or(0); }

        let doc = self.prepared_doc(json, opts)?;
        let names: std::collections::HashMap<&str, &str> = doc.vocab.iter().map(|v| (v.id.as_str(), v.ja.as_str())).collect();
        let name = |id: &Option<String>| id.as_deref().and_then(|i| names.get(i).copied()).unwrap_or("").to_string();
        let existing = self.existing_fingerprints()?;
        let rows = doc
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| PreviewRow {
                index: i,
                date: s.date.clone(),
                date_prec: s.date_prec.clone(),
                place: name(&s.place_id),
                kind: s.kind.clone(),
                summary: s
                    .temae
                    .iter()
                    .map(|t| {
                        let n = name(&t.temae_alias_id.clone().or(t.temae_id.clone()));
                        if n.is_empty() { t.temae_free.clone().unwrap_or_default() } else { n }
                    })
                    .collect::<Vec<_>>()
                    .join(" / "),
                n_temae: s.temae.len(),
                duplicate_of: existing.get(&fingerprint(s)).copied(),
            })
            .collect();
        let mut vocab_new = 0;
        for v in &doc.vocab {
            if self.vocab_get(&v.id)?.is_none() { vocab_new += 1; }
        }
        Ok(ImportPreview { persons, rows, vocab_new, schema_version: doc.schema_version })
    }

    /// Import with options (self swap, explicit selection). `import_json` is the
    /// no-options form.
    pub fn import_json_with(&self, json: &str, opts: &ImportOptions) -> Result<ImportReport> {
        let doc = self.prepared_doc(json, opts)?;
        self.import_doc(doc, opts.include.as_deref())
    }

    pub fn import_json(&self, json: &str) -> Result<ImportReport> {
        let doc = parse(json)?;
        self.import_doc(doc, None)
    }

    fn import_doc(&self, doc: ExportDoc, include: Option<&[usize]>) -> Result<ImportReport> {
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

        let existing = self.existing_fingerprints()?;
        for (i, mut s) in doc.sessions.into_iter().enumerate() {
            let wanted = match include {
                Some(list) => list.contains(&i),
                None => !existing.contains_key(&fingerprint(&s)),
            };
            if !wanted {
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
    use crate::model::{Temae, Vocab};
    use crate::pack::{Pack, URASENKE_YAML};

    #[test]
    fn roundtrip_export_import_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        let me = db.self_person_id().unwrap();
        db.session_save(&Session {
            date: Some("2026-05-10".into()),
            temae: vec![Temae { temae_id: Some("temae.urasenke.karamono".into()), host_id: Some(me), my_role: Some("teishu".into()), ..Default::default() }],
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

    #[test]
    fn preview_and_swap_self() {
        // Author's diary: author hosted karamono with "Anna" as main guest; Anna hosted hiradate with author watching.
        let src = Db::open_in_memory().unwrap();
        src.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        let author = src.self_person_id().unwrap();
        src.vocab_upsert(&Vocab { id: "person.anna".into(), kind: "person".into(), ja: "Anna".into(), romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None }).unwrap();
        src.session_save(&Session {
            date: Some("2026-05-01".into()),
            temae: vec![
                Temae { temae_id: Some("temae.urasenke.karamono".into()), host_id: Some(author.clone()), my_role: Some("teishu".into()),
                        persons: vec![crate::model::TemaePerson { person_id: Some("person.anna".into()), name_free: None, role: "kyaku".into(), seat: Some(1), is_last: false, role_note: None }],
                        ..Default::default() },
                Temae { temae_id: Some("temae.urasenke.hiradate".into()), host_id: Some("person.anna".into()), my_role: Some("kengaku".into()), ..Default::default() },
            ],
            ..Default::default()
        }).unwrap();
        let json = src.export_json().unwrap();

        // Anna imports it as her own diary
        let dst = Db::open_in_memory().unwrap();
        dst.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        let me = dst.self_person_id().unwrap();
        let opts = ImportOptions { self_as: Some("person.anna".into()), original_self_name: Some("佐藤さん".into()), include: None };
        let pv = dst.import_preview(&json, &opts).unwrap();
        assert!(pv.persons.iter().any(|p| p.id == "person.anna" && p.n == 2));
        assert_eq!(pv.rows.len(), 1);
        assert!(pv.rows[0].duplicate_of.is_none());
        assert!(pv.rows[0].summary.contains("唐物"));

        let rep = dst.import_json_with(&json, &opts).unwrap();
        assert_eq!(rep.sessions_added, 1);
        let s = dst.session_get(dst.session_list(1, 0).unwrap()[0].id).unwrap().unwrap();
        // temae 1: author hosted, I (Anna) was main guest
        assert_ne!(s.temae[0].host_id.as_deref(), Some(me.as_str()));
        assert_eq!(s.temae[0].my_role.as_deref(), Some("shokyaku"));
        assert_eq!(s.temae[0].my_seat, Some(1));
        assert!(s.temae[0].persons.is_empty());
        // temae 2: I hosted, the author watched
        assert_eq!(s.temae[1].host_id.as_deref(), Some(me.as_str()));
        assert_eq!(s.temae[1].my_role.as_deref(), Some("teishu"));
        assert_eq!(s.temae[1].persons.len(), 1);
        assert_eq!(s.temae[1].persons[0].role, "kengaku");
        let author_person = dst.vocab_get(s.temae[0].host_id.as_ref().unwrap()).unwrap().unwrap();
        assert_eq!(author_person.ja, "佐藤さん");
        // no second "self"
        assert_eq!(dst.vocab_list("person", true).unwrap().iter().filter(|v| v.meta.as_ref().and_then(|m| m.get("is_self")).is_some()).count(), 1);

        // importing again: preview flags the duplicate; explicit include re-imports anyway
        let pv2 = dst.import_preview(&json, &opts).unwrap();
        assert!(pv2.rows[0].duplicate_of.is_some());
        let rep2 = dst.import_json_with(&json, &opts).unwrap();
        assert_eq!(rep2.sessions_added, 0);
        let rep3 = dst.import_json_with(&json, &ImportOptions { include: Some(vec![0]), ..opts.clone() }).unwrap();
        assert_eq!(rep3.sessions_added, 1);
    }

    /// An export from v0.2 (schema 1, unscoped ids) imports into the current app without
    /// creating duplicate vocabulary.
    #[test]
    fn schema1_export_upgrades_on_import() {
        const OLD: &str = include_str!("../testdata/sample-v1.json");
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        db.self_person_id().unwrap();
        let before = db.vocab_list("temae", true).unwrap().len();
        let rep = db.import_json(OLD).unwrap();
        assert!(rep.sessions_added > 100);
        // no unscoped pack rows sneaked in
        assert!(db.vocab_get("temae.karamono").unwrap().is_none());
        assert_eq!(db.vocab_list("temae", true).unwrap().len(), before);
        let s = db.session_list(1, 0).unwrap();
        let full = db.session_get(s[0].id).unwrap().unwrap();
        assert!(full.temae[0].temae_id.as_deref().unwrap().starts_with("temae.urasenke."));
    }

    /// The bundled sample data must import into a fresh app (pack already installed)
    /// and be usable by every statistics query.
    #[test]
    fn sample_data_imports() {
        const SAMPLE: &str = include_str!("../../../samples/sample-data.json");
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        db.self_person_id().unwrap();
        let rep = db.import_json(SAMPLE).unwrap();
        assert!(rep.sessions_added > 100, "{rep:?}");
        assert_eq!(rep.sessions_skipped, 0);
        let f = crate::stats::StatsFilter::default();
        assert!(db.stats_coverage(&f).unwrap().len() > 20);
        assert!(!db.stats_monthly(&f).unwrap().is_empty());
        assert!(!db.stats_cross("temae", "tana", true, &f).unwrap().is_empty());
        for d in ["teacher", "place", "host", "with"] {
            assert!(!db.stats_by(d, &f).unwrap().is_empty(), "{d}");
        }
        assert!(!db.vocab_ranking("person").unwrap().is_empty());
        // importing again is a no-op
        let rep2 = db.import_json(SAMPLE).unwrap();
        assert_eq!(rep2.sessions_added, 0);
    }
}
