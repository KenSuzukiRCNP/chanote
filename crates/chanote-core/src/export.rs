//! Export: self-contained JSON (vocab + all sessions) and a flat CSV (one row per temae).

use crate::model::{Session, Vocab};
use crate::{Db, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportDoc {
    pub app: String,
    pub schema_version: u32,
    pub exported_at: String,
    pub vocab: Vec<Vocab>,
    pub sessions: Vec<Session>,
}

impl Db {
    fn all_sessions(&self) -> Result<Vec<Session>> {
        let ids: Vec<i64> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT id FROM session ORDER BY date, id")?;
            let v = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
            v
        };
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(s) = self.session_get(id)? {
                out.push(s);
            }
        }
        Ok(out)
    }

    fn all_vocab(&self) -> Result<Vec<Vocab>> {
        let mut out = Vec::new();
        for kind in ["school", "temae-cat", "temae", "place", "person", "tana", "hearth", "tool-cat", "tool"] {
            out.extend(self.vocab_list(kind, true)?);
        }
        Ok(out)
    }

    pub fn export_json(&self) -> Result<String> {
        let doc = ExportDoc {
            app: "chanote".into(),
            schema_version: SCHEMA_VERSION,
            exported_at: now_iso(),
            vocab: self.all_vocab()?,
            sessions: self.all_sessions()?,
        };
        Ok(serde_json::to_string_pretty(&doc)?)
    }

    /// One row per temae. Names are Japanese (`ja`); id columns are appended for analysis.
    pub fn export_csv(&self) -> Result<String> {
        let names: HashMap<String, String> =
            self.all_vocab()?.into_iter().map(|v| (v.id, v.ja)).collect();
        let name = |id: &Option<String>| -> String {
            match id {
                None => String::new(),
                Some(i) => names.get(i).cloned().unwrap_or_else(|| i.clone()),
            }
        };

        let mut w = String::new();
        let header = [
            "session_id", "date", "date_prec", "place", "session_kind", "online", "teachers",
            "ord", "time", "temae", "temae_alias", "temae_free", "host", "my_role", "my_role_note",
            "my_seat", "hearth", "katte", "room", "tea", "tana", "tana_free", "kinin", "tsukimono",
            "teacher", "participants", "tools", "note",
            "temae_id", "temae_alias_id", "host_id", "tana_id", "place_id", "teacher_id",
        ];
        push_row(&mut w, header.iter().map(|s| s.to_string()));

        for s in self.all_sessions()? {
            let sid = s.id.unwrap_or(0);
            let teachers = s
                .persons
                .iter()
                .filter(|p| p.role == "teacher")
                .map(|p| p.name_free.clone().unwrap_or_else(|| name(&p.person_id)))
                .collect::<Vec<_>>()
                .join("; ");
            for (i, t) in s.temae.iter().enumerate() {
                let participants = t
                    .persons
                    .iter()
                    .map(|p| {
                        let n = p.name_free.clone().unwrap_or_else(|| name(&p.person_id));
                        match p.seat {
                            Some(seat) => format!("{n}@{seat}"),
                            None => n,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let tools = t
                    .tools
                    .iter()
                    .map(|x| x.name_free.clone().unwrap_or_else(|| name(&x.tool_id)))
                    .collect::<Vec<_>>()
                    .join("; ");
                let host = t.host_free.clone().unwrap_or_else(|| name(&t.host_id));
                let row: Vec<String> = vec![
                    sid.to_string(),
                    s.date.clone().unwrap_or_default(),
                    s.date_prec.clone(),
                    name(&s.place_id),
                    s.kind.clone(),
                    if s.online { "1".into() } else { "0".into() },
                    teachers.clone(),
                    (i + 1).to_string(),
                    t.time.clone().unwrap_or_default(),
                    name(&t.temae_id),
                    name(&t.temae_alias_id),
                    t.temae_free.clone().unwrap_or_default(),
                    host,
                    t.my_role.clone().unwrap_or_default(),
                    t.my_role_note.clone().unwrap_or_default(),
                    t.my_seat.map(|x| x.to_string()).unwrap_or_default(),
                    t.hearth.clone().unwrap_or_default(),
                    t.katte.clone().unwrap_or_default(),
                    t.room.clone().unwrap_or_default(),
                    t.tea.clone().unwrap_or_default(),
                    match t.tana_id.as_deref() {
                        Some("-") => "-".into(),
                        Some("?") => "?".into(),
                        _ => name(&t.tana_id),
                    },
                    t.tana_free.clone().unwrap_or_default(),
                    t.kinin.clone().unwrap_or_default(),
                    t.tsukimono.join("; "),
                    name(&t.teacher_id),
                    participants,
                    tools,
                    t.note.clone().unwrap_or_default(),
                    t.temae_id.clone().unwrap_or_default(),
                    t.temae_alias_id.clone().unwrap_or_default(),
                    t.host_id.clone().unwrap_or_default(),
                    t.tana_id.clone().unwrap_or_default(),
                    s.place_id.clone().unwrap_or_default(),
                    t.teacher_id.clone().unwrap_or_default(),
                ];
                push_row(&mut w, row.into_iter());
            }
        }
        Ok(w)
    }
}

fn push_row(w: &mut String, cells: impl Iterator<Item = String>) {
    let mut first = true;
    for c in cells {
        if !first {
            w.push(',');
        }
        first = false;
        let needs_quote = c.contains([',', '"', '\n', '\r']);
        if needs_quote {
            w.push('"');
            w.push_str(&c.replace('"', "\"\""));
            w.push('"');
        } else {
            w.push_str(&c);
        }
    }
    w.push_str("\r\n");
}

fn now_iso() -> String {
    // avoid a chrono dependency: SQLite has a clock
    let conn = rusqlite::Connection::open_in_memory().expect("mem db");
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |r| r.get(0))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Temae, Vocab};
    use crate::pack::{Pack, URASENKE_YAML};

    #[test]
    fn json_and_csv_export() {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        let me = db.self_person_id().unwrap();
        db.vocab_upsert(&Vocab {
            id: "place.shofu-an".into(), kind: "place".into(), ja: "松風庵".into(),
            romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(),
            archived: false, meta: None,
        }).unwrap();
        db.session_save(&Session {
            date: Some("2026-05-10".into()),
            place_id: Some("place.shofu-an".into()),
            online: true,
            note: Some("memo, with \"quotes\"".into()),
            temae: vec![Temae {
                temae_id: Some("temae.kagetsu".into()),
                temae_alias_id: Some("temae.kagetsu.koicha".into()),
                host_id: Some(me.clone()),
                my_role: Some("teishu".into()),
                my_role_note: Some("月".into()),
                hearth: Some("furo".into()),
                tea: Some("koicha".into()),
                tana_id: Some("-".into()),
                tsukimono: vec!["koicha".into()],
                note: Some("line1\nline2".into()),
                ..Default::default()
            }],
            ..Default::default()
        }).unwrap();

        let json = db.export_json().unwrap();
        let doc: ExportDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(doc.schema_version, SCHEMA_VERSION);
        assert_eq!(doc.sessions.len(), 1);
        assert!(doc.sessions[0].online);
        assert!(doc.vocab.iter().any(|v| v.id == "temae.kagetsu"));

        let csv = db.export_csv().unwrap();
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert!(lines[0].starts_with("session_id,date,"));
        assert!(lines[1].contains("松風庵"));
        assert!(lines[1].contains("花月之式,濃茶付花月"));
        assert!(lines[1].contains("自分,teishu,月"));
        assert!(lines[1].contains("\"line1\nline2\""));
    }
}
