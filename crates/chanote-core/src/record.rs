use crate::model::{Session, SessionPerson, SessionSummary, Temae, TemaePerson, TemaeTool};
use crate::{Db, Error, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

/// Filter for the records list. Every field is optional; lists are AND-ed
/// (all listed persons/tools must appear somewhere in the session).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionFilter {
    /// main temae ids OR alias ids; a session matches if any of its temae is in the set
    #[serde(default)]
    pub temae: Vec<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub place_id: Option<String>,
    pub teacher_id: Option<String>,
    /// person ids (host, teacher, guest, participant — anywhere in the session)
    #[serde(default)]
    pub persons: Vec<String>,
    /// tool ids
    #[serde(default)]
    pub tools: Vec<String>,
    /// "teishu" to keep only sessions where I hosted at least one of the (filtered) temae
    pub my_role: Option<String>,
    /// free-text search in notes (session note, temae note)
    pub text: Option<String>,
    /// session form: temae | chaji | chakai | koshukai | "?"
    #[serde(default)]
    pub form: Option<String>,
    /// event kind (vocab id)
    #[serde(default)]
    pub event_kind: Option<String>,
    /// Some(true) = lessons only, Some(false) = real events only
    #[serde(default)]
    pub practice: Option<bool>,
}

impl SessionFilter {
    pub fn is_empty(&self) -> bool {
        self == &SessionFilter::default()
    }
}

impl Db {
    /// Home list, newest first. `limit`/`offset` for paging.
    pub fn session_list(&self, limit: i64, offset: i64) -> Result<Vec<SessionSummary>> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT s.id, s.date, s.date_prec, s.place_id, s.form, s.event_kind, s.practice, s.my_role, s.online,
                    (SELECT COUNT(*) FROM temae t WHERE t.session_id = s.id) AS n
             FROM session s
             ORDER BY s.date DESC NULLS LAST, s.id DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = st.query_map(params![limit, offset], |r| {
            Ok(SessionSummary {
                id: r.get(0)?,
                date: r.get(1)?,
                date_prec: r.get(2)?,
                place_id: r.get(3)?,
                form: r.get(4)?,
                event_kind: r.get(5)?,
                practice: r.get::<_, i64>(6)? != 0,
                my_role: r.get(7)?,
                online: r.get::<_, i64>(8)? != 0,
                temae_count: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Filtered list, newest first. Returns summaries like `session_list`.
    pub fn session_search(&self, f: &SessionFilter, limit: i64, offset: i64) -> Result<Vec<SessionSummary>> {
        let mut sql = String::from(
            "SELECT s.id, s.date, s.date_prec, s.place_id, s.form, s.event_kind, s.practice, s.my_role, s.online,
                    (SELECT COUNT(*) FROM temae t WHERE t.session_id = s.id) AS n
             FROM session s WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(x) = &f.since { sql.push_str(" AND s.date >= ?"); args.push(Box::new(x.clone())); }
        if let Some(x) = &f.until { sql.push_str(" AND s.date <= ?"); args.push(Box::new(x.clone())); }
        if let Some(x) = &f.place_id { sql.push_str(" AND s.place_id = ?"); args.push(Box::new(x.clone())); }
        if let Some(x) = &f.form { sql.push_str(" AND s.form = ?"); args.push(Box::new(x.clone())); }
        if let Some(x) = &f.event_kind { sql.push_str(" AND s.event_kind = ?"); args.push(Box::new(x.clone())); }
        if let Some(x) = f.practice { sql.push_str(" AND s.practice = ?"); args.push(Box::new(x as i64)); }
        if let Some(x) = &f.teacher_id {
            sql.push_str(" AND (EXISTS (SELECT 1 FROM session_person sp WHERE sp.session_id = s.id AND sp.role = 'teacher' AND sp.person_id = ?)
                              OR EXISTS (SELECT 1 FROM temae t WHERE t.session_id = s.id AND t.teacher_id = ?))");
            args.push(Box::new(x.clone()));
            args.push(Box::new(x.clone()));
        }
        if !f.temae.is_empty() {
            let marks = vec!["?"; f.temae.len()].join(",");
            let role = if f.my_role.is_some() { " AND t.my_role = ?" } else { "" };
            sql.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM temae t WHERE t.session_id = s.id AND (t.temae_id IN ({marks}) OR t.temae_alias_id IN ({marks})){role})"
            ));
            for x in &f.temae { args.push(Box::new(x.clone())); }
            for x in &f.temae { args.push(Box::new(x.clone())); }
            if let Some(r) = &f.my_role { args.push(Box::new(r.clone())); }
        } else if let Some(r) = &f.my_role {
            sql.push_str(" AND (s.my_role = ? OR EXISTS (SELECT 1 FROM temae t WHERE t.session_id = s.id AND t.my_role = ?))");
            args.push(Box::new(r.clone()));
            args.push(Box::new(r.clone()));
        }
        for p in &f.persons {
            sql.push_str(
                " AND (EXISTS (SELECT 1 FROM session_person sp WHERE sp.session_id = s.id AND sp.person_id = ?)
                   OR EXISTS (SELECT 1 FROM temae t WHERE t.session_id = s.id AND (t.host_id = ? OR t.teacher_id = ?))
                   OR EXISTS (SELECT 1 FROM temae_person tp JOIN temae t ON t.id = tp.temae_id WHERE t.session_id = s.id AND tp.person_id = ?))",
            );
            for _ in 0..4 { args.push(Box::new(p.clone())); }
        }
        for tool in &f.tools {
            sql.push_str(" AND EXISTS (SELECT 1 FROM temae_tool tt JOIN temae t ON t.id = tt.temae_id WHERE t.session_id = s.id AND tt.tool_id = ?)");
            args.push(Box::new(tool.clone()));
        }
        if let Some(x) = f.text.as_ref().filter(|x| !x.trim().is_empty()) {
            let like = format!("%{}%", x.trim());
            sql.push_str(" AND (s.note LIKE ? OR EXISTS (SELECT 1 FROM temae t WHERE t.session_id = s.id AND (t.note LIKE ? OR t.my_role_note LIKE ?)))");
            for _ in 0..3 { args.push(Box::new(like.clone())); }
        }
        sql.push_str(" ORDER BY s.date DESC NULLS LAST, s.id DESC LIMIT ? OFFSET ?");
        args.push(Box::new(limit));
        args.push(Box::new(offset));

        let conn = self.conn();
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map(rusqlite::params_from_iter(args.iter().map(|b| b.as_ref())), |r| {
            Ok(SessionSummary {
                id: r.get(0)?,
                date: r.get(1)?,
                date_prec: r.get(2)?,
                place_id: r.get(3)?,
                form: r.get(4)?,
                event_kind: r.get(5)?,
                practice: r.get::<_, i64>(6)? != 0,
                my_role: r.get(7)?,
                online: r.get::<_, i64>(8)? != 0,
                temae_count: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn session_get(&self, id: i64) -> Result<Option<Session>> {
        let conn = self.conn();
        let head = conn
            .query_row(
                "SELECT id, date, date_prec, place_id, place_note, form, event_kind, practice, my_role, my_seat, online, note
                 FROM session WHERE id = ?1",
                [id],
                |r| {
                    Ok(Session {
                        id: Some(r.get(0)?),
                        date: r.get(1)?,
                        date_prec: r.get(2)?,
                        place_id: r.get(3)?,
                        place_note: r.get(4)?,
                        form: r.get(5)?,
                        event_kind: r.get(6)?,
                        practice: r.get::<_, i64>(7)? != 0,
                        my_role: r.get(8)?,
                        my_seat: r.get(9)?,
                        online: r.get::<_, i64>(10)? != 0,
                        note: r.get(11)?,
                        persons: vec![],
                        temae: vec![],
                        kind: None,
                    })
                },
            )
            .optional()?;
        let Some(mut s) = head else { return Ok(None) };

        let mut st = conn.prepare(
            "SELECT person_id, name_free, role, seat, is_last, role_note FROM session_person WHERE session_id = ?1 ORDER BY ord, id",
        )?;
        s.persons = st
            .query_map([id], |r| {
                Ok(SessionPerson {
                    person_id: r.get(0)?,
                    name_free: r.get(1)?,
                    role: r.get(2)?,
                    seat: r.get(3)?,
                    is_last: r.get::<_, i64>(4)? != 0,
                    role_note: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        let mut st = conn.prepare(
            "SELECT id, time, temae_id, temae_alias_id, temae_free, host_id, host_free, teacher_id,
                    my_role, my_role_note, my_seat, hearth, katte, room, tea, tana_id, tana_free, kinin, note
             FROM temae WHERE session_id = ?1 ORDER BY ord, id",
        )?;
        let mut temae: Vec<Temae> = st
            .query_map([id], |r| {
                Ok(Temae {
                    id: Some(r.get(0)?),
                    time: r.get(1)?,
                    temae_id: r.get(2)?,
                    temae_alias_id: r.get(3)?,
                    temae_free: r.get(4)?,
                    host_id: r.get(5)?,
                    host_free: r.get(6)?,
                    teacher_id: r.get(7)?,
                    my_role: r.get(8)?,
                    my_role_note: r.get(9)?,
                    my_seat: r.get(10)?,
                    hearth: r.get(11)?,
                    katte: r.get(12)?,
                    room: r.get(13)?,
                    tea: r.get(14)?,
                    tana_id: r.get(15)?,
                    tana_free: r.get(16)?,
                    kinin: r.get(17)?,
                    note: r.get(18)?,
                    persons: vec![],
                    tsukimono: vec![],
                    tools: vec![],
                })
            })?
            .collect::<rusqlite::Result<_>>()?;

        for t in temae.iter_mut() {
            let tid = t.id.unwrap();
            let mut st = conn.prepare(
                "SELECT person_id, name_free, role, seat, is_last, role_note
                 FROM temae_person WHERE temae_id = ?1 ORDER BY seat NULLS LAST, id",
            )?;
            t.persons = st
                .query_map([tid], |r| {
                    Ok(TemaePerson {
                        person_id: r.get(0)?,
                        name_free: r.get(1)?,
                        role: r.get(2)?,
                        seat: r.get(3)?,
                        is_last: r.get::<_, i64>(4)? != 0,
                        role_note: r.get(5)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            let mut st =
                conn.prepare("SELECT tsukimono FROM temae_tsukimono WHERE temae_id = ?1 ORDER BY tsukimono")?;
            t.tsukimono = st.query_map([tid], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            let mut st = conn.prepare(
                "SELECT tool_id, name_free, external_ref FROM temae_tool WHERE temae_id = ?1 ORDER BY ord, id",
            )?;
            t.tools = st
                .query_map([tid], |r| {
                    Ok(TemaeTool { tool_id: r.get(0)?, name_free: r.get(1)?, external_ref: r.get(2)? })
                })?
                .collect::<rusqlite::Result<_>>()?;
        }
        s.temae = temae;
        Ok(Some(s))
    }

    /// Insert (id == None) or fully replace (id == Some) a session with its children.
    /// Returns the session id.
    pub fn session_save(&self, s: &Session) -> Result<i64> {
        validate(s)?;
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let id = match s.id {
            None => {
                tx.execute(
                    "INSERT INTO session (date, date_prec, place_id, place_note, form, event_kind, practice, my_role, my_seat, online, note)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![s.date, s.date_prec, s.place_id, s.place_note, s.form, s.event_kind, s.practice as i64,
                            s.my_role, s.my_seat, s.online as i64, s.note],
                )?;
                tx.last_insert_rowid()
            }
            Some(id) => {
                let n = tx.execute(
                    "UPDATE session SET date=?2, date_prec=?3, place_id=?4, place_note=?5, form=?6, event_kind=?7, practice=?8,
                            my_role=?9, my_seat=?10, online=?11, note=?12,
                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                     WHERE id = ?1",
                    params![id, s.date, s.date_prec, s.place_id, s.place_note, s.form, s.event_kind, s.practice as i64,
                            s.my_role, s.my_seat, s.online as i64, s.note],
                )?;
                if n == 0 {
                    return Err(Error::Other(format!("session {id} not found")));
                }
                // children are replaced wholesale (temae ids are not stable across saves)
                tx.execute("DELETE FROM session_person WHERE session_id = ?1", [id])?;
                tx.execute("DELETE FROM temae WHERE session_id = ?1", [id])?;
                id
            }
        };
        for (i, p) in s.persons.iter().enumerate() {
            tx.execute(
                "INSERT INTO session_person (session_id, person_id, name_free, role, ord, seat, is_last, role_note)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![id, p.person_id, p.name_free, p.role, i as i64, p.seat, p.is_last as i64, p.role_note],
            )?;
        }
        for (i, t) in s.temae.iter().enumerate() {
            insert_temae(&tx, id, i as i64, t)?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn session_delete(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM session WHERE id = ?1", [id])?;
        Ok(())
    }
}

fn validate(s: &Session) -> Result<()> {
    match s.date_prec.as_str() {
        "day" | "month" | "year" | "unknown" => {}
        p => return Err(Error::Other(format!("bad date_prec {p}"))),
    }
    if !crate::model::SESSION_FORMS.contains(&s.form.as_str()) {
        return Err(Error::Other(format!("bad session form {}", s.form)));
    }
    if let Some(r) = s.my_role.as_deref() {
        if !crate::model::MY_ROLES.contains(&r) {
            return Err(Error::Other(format!("bad my_role {r}")));
        }
    }
    for p in &s.persons {
        if !crate::model::SESSION_ROLES.contains(&p.role.as_str()) {
            return Err(Error::Other(format!("bad session person role {}", p.role)));
        }
    }
    for t in &s.temae {
        if let Some(r) = t.my_role.as_deref() {
            if !crate::model::MY_ROLES.contains(&r) {
                return Err(Error::Other(format!("bad temae my_role {r}")));
            }
        }
    }
    if s.date_prec != "unknown" && s.date.is_none() {
        return Err(Error::Other("date required unless date_prec = unknown".into()));
    }
    Ok(())
}

fn insert_temae(tx: &Transaction, session_id: i64, ord: i64, t: &Temae) -> Result<()> {
    tx.execute(
        "INSERT INTO temae (session_id, ord, time, temae_id, temae_alias_id, temae_free, host_id, host_free,
                            teacher_id, my_role, my_role_note, my_seat, hearth, katte, room, tea,
                            tana_id, tana_free, kinin, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            session_id, ord, t.time, t.temae_id, t.temae_alias_id, t.temae_free, t.host_id, t.host_free,
            t.teacher_id, t.my_role, t.my_role_note, t.my_seat, t.hearth, t.katte, t.room, t.tea,
            t.tana_id, t.tana_free, t.kinin, t.note
        ],
    )?;
    let tid = tx.last_insert_rowid();
    for p in &t.persons {
        tx.execute(
            "INSERT INTO temae_person (temae_id, person_id, name_free, role, seat, is_last, role_note)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![tid, p.person_id, p.name_free, p.role, p.seat, p.is_last as i64, p.role_note],
        )?;
    }
    for k in &t.tsukimono {
        tx.execute(
            "INSERT OR IGNORE INTO temae_tsukimono (temae_id, tsukimono) VALUES (?1, ?2)",
            params![tid, k],
        )?;
    }
    for (i, tool) in t.tools.iter().enumerate() {
        tx.execute(
            "INSERT INTO temae_tool (temae_id, ord, tool_id, name_free, external_ref) VALUES (?1,?2,?3,?4,?5)",
            params![tid, i as i64, tool.tool_id, tool.name_free, tool.external_ref],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vocab;

    fn seed(db: &Db) {
        for (id, kind, ja) in [
            ("place.shofu-an", "place", "松風庵"),
            ("person.yamada", "person", "山田先生"),
            ("person.sato", "person", "佐藤さん"),
            ("temae.urasenke.shin-no-gyo", "temae", "真之行台子"),
            ("temae.urasenke.karamono", "temae", "唐物"),
        ] {
            db.vocab_upsert(&Vocab {
                id: id.into(),
                kind: kind.into(),
                ja: ja.into(),
                romaji: None,
                en: None,
                parent_id: None,
                sort: None,
                origin: "user".into(),
                archived: false,
                meta: None,
            })
            .unwrap();
        }
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        seed(&db);
        let me = db.self_person_id().unwrap();

        // 10.05.2026 松風庵 講習会 / 真乃行台子 佐藤さん 山田先生 客(自分) / 真乃行台子 自分 山田先生
        let s = Session {
            id: None,
            date: Some("2026-05-10".into()),
            date_prec: "day".into(),
            place_id: Some("place.shofu-an".into()),
            place_note: None,
            form: "koshukai".into(),
            online: true,
            note: None,
            persons: vec![SessionPerson {
                person_id: Some("person.yamada".into()),
                name_free: None,
                role: "teacher".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let s = Session {
            temae: vec![
                Temae {
                    temae_id: Some("temae.urasenke.shin-no-gyo".into()),
                    host_id: Some("person.sato".into()),
                    teacher_id: Some("person.yamada".into()),
                    my_role: Some("kyaku".into()),
                    hearth: Some("furo".into()),
                    tea: Some("koicha".into()),
                    ..Default::default()
                },
                Temae {
                    temae_id: Some("temae.urasenke.shin-no-gyo".into()),
                    host_id: Some(me.clone()),
                    my_role: Some("teishu".into()),
                    hearth: Some("furo".into()),
                    persons: vec![TemaePerson {
                        person_id: Some("person.sato".into()),
                        name_free: None,
                        role: "kyaku".into(),
                        seat: Some(1),
                        is_last: false,
                        role_note: None,
                    }],
                    ..Default::default()
                },
            ],
            ..s
        };
        let id = db.session_save(&s).unwrap();
        let back = db.session_get(id).unwrap().unwrap();
        assert_eq!(back.form, "koshukai");
        assert!(back.practice);
        assert!(back.online);
        assert_eq!(back.temae.len(), 2);
        assert_eq!(back.temae[0].my_role.as_deref(), Some("kyaku"));
        assert_eq!(back.temae[1].persons[0].seat, Some(1));

        // update: drop second temae
        let mut edited = back.clone();
        edited.temae.pop();
        db.session_save(&edited).unwrap();
        let back2 = db.session_get(id).unwrap().unwrap();
        assert_eq!(back2.temae.len(), 1);

        let list = db.session_list(10, 0).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].temae_count, 1);

        db.session_delete(id).unwrap();
        assert!(db.session_get(id).unwrap().is_none());
    }

    #[test]
    fn search_filters() {
        let db = Db::open_in_memory().unwrap();
        seed(&db);
        db.vocab_upsert(&Vocab { id: "tool.bowl".into(), kind: "tool".into(), ja: "茶碗".into(), romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None }).unwrap();
        let me = db.self_person_id().unwrap();
        let mk = |date: &str, temae: &str, host: &str, role: &str, note: Option<&str>, tool: bool| Session {
            date: Some(date.into()), place_id: Some("place.shofu-an".into()),
            persons: vec![SessionPerson { person_id: Some("person.yamada".into()), role: "teacher".into(), ..Default::default() }],
            temae: vec![Temae {
                temae_id: Some(temae.into()), host_id: Some(host.into()), my_role: Some(role.into()),
                note: note.map(Into::into),
                tools: if tool { vec![TemaeTool { tool_id: Some("tool.bowl".into()), name_free: None, external_ref: None }] } else { vec![] },
                ..Default::default()
            }],
            ..Default::default()
        };
        db.session_save(&mk("2026-01-10", "temae.urasenke.shin-no-gyo", &me, "teishu", Some("柄杓の扱いを注意された"), true)).unwrap();
        db.session_save(&mk("2026-02-10", "temae.urasenke.shin-no-gyo", "person.sato", "kyaku", None, false)).unwrap();
        db.session_save(&mk("2026-03-10", "temae.urasenke.karamono", &me, "teishu", None, false)).unwrap();

        let all = db.session_search(&SessionFilter::default(), 100, 0).unwrap();
        assert_eq!(all.len(), 3);
        let f = SessionFilter { temae: vec!["temae.urasenke.shin-no-gyo".into()], ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 2);
        let f = SessionFilter { temae: vec!["temae.urasenke.shin-no-gyo".into()], my_role: Some("teishu".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { persons: vec!["person.sato".into()], ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { persons: vec!["person.sato".into(), "person.yamada".into()], ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { tools: vec!["tool.bowl".into()], ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { since: Some("2026-02-01".into()), until: Some("2026-02-28".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { text: Some("柄杓".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { teacher_id: Some("person.yamada".into()), place_id: Some("place.shofu-an".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 3);

        // a real chaji where I was main guest: form / practice / event kind / session my_role filters
        db.session_save(&Session {
            date: Some("2026-04-10".into()), form: "chaji".into(), event_kind: Some("event.urasenke.shogo".into()),
            practice: false, my_role: Some("shokyaku".into()), my_seat: Some(1),
            persons: vec![
                SessionPerson { person_id: Some("person.sato".into()), role: "host".into(), ..Default::default() },
                SessionPerson { person_id: Some("person.yamada".into()), role: "guest".into(), seat: Some(2), is_last: true, ..Default::default() },
            ],
            temae: vec![Temae { temae_id: Some("temae.urasenke.karamono".into()), host_id: Some("person.sato".into()), my_role: Some("shokyaku".into()), ..Default::default() }],
            ..Default::default()
        }).unwrap();
        let f = SessionFilter { form: Some("chaji".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { practice: Some(false), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { practice: Some(true), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 3);
        let f = SessionFilter { event_kind: Some("event.urasenke.shogo".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        let f = SessionFilter { my_role: Some("shokyaku".into()), ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 1);
        // the roster is searchable like temae participants
        let f = SessionFilter { persons: vec!["person.sato".into()], ..Default::default() };
        assert_eq!(db.session_search(&f, 100, 0).unwrap().len(), 2);
        let back = db.session_get(db.session_search(&SessionFilter { form: Some("chaji".into()), ..Default::default() }, 1, 0).unwrap()[0].id).unwrap().unwrap();
        assert_eq!(back.persons[1].seat, Some(2));
        assert!(back.persons[1].is_last);
        assert_eq!(back.my_seat, Some(1));
        // bad values are rejected
        assert!(db.session_save(&Session { date: Some("2026-01-01".into()), form: "picnic".into(), ..Default::default() }).is_err());
        assert!(db.session_save(&Session { date: Some("2026-01-01".into()), my_role: Some("boss".into()), ..Default::default() }).is_err());
    }

    #[test]
    fn unknown_date_allowed() {
        let db = Db::open_in_memory().unwrap();
        let s = Session { date_prec: "unknown".into(), ..Default::default() };
        db.session_save(&s).unwrap();
        let s = Session { date_prec: "day".into(), ..Default::default() };
        assert!(db.session_save(&s).is_err());
    }
}
