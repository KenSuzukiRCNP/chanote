use crate::model::{Session, SessionPerson, SessionSummary, Temae, TemaePerson, TemaeTool};
use crate::{Db, Error, Result};
use rusqlite::{params, OptionalExtension, Transaction};

impl Db {
    /// Home list, newest first. `limit`/`offset` for paging.
    pub fn session_list(&self, limit: i64, offset: i64) -> Result<Vec<SessionSummary>> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT s.id, s.date, s.date_prec, s.place_id, s.kind, s.online,
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
                kind: r.get(4)?,
                online: r.get::<_, i64>(5)? != 0,
                temae_count: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn session_get(&self, id: i64) -> Result<Option<Session>> {
        let conn = self.conn();
        let head = conn
            .query_row(
                "SELECT id, date, date_prec, place_id, place_note, kind, online, note FROM session WHERE id = ?1",
                [id],
                |r| {
                    Ok(Session {
                        id: Some(r.get(0)?),
                        date: r.get(1)?,
                        date_prec: r.get(2)?,
                        place_id: r.get(3)?,
                        place_note: r.get(4)?,
                        kind: r.get(5)?,
                        online: r.get::<_, i64>(6)? != 0,
                        note: r.get(7)?,
                        persons: vec![],
                        temae: vec![],
                    })
                },
            )
            .optional()?;
        let Some(mut s) = head else { return Ok(None) };

        let mut st = conn.prepare(
            "SELECT person_id, name_free, role FROM session_person WHERE session_id = ?1 ORDER BY ord, id",
        )?;
        s.persons = st
            .query_map([id], |r| {
                Ok(SessionPerson { person_id: r.get(0)?, name_free: r.get(1)?, role: r.get(2)? })
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
                    "INSERT INTO session (date, date_prec, place_id, place_note, kind, online, note)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![s.date, s.date_prec, s.place_id, s.place_note, s.kind, s.online as i64, s.note],
                )?;
                tx.last_insert_rowid()
            }
            Some(id) => {
                let n = tx.execute(
                    "UPDATE session SET date=?2, date_prec=?3, place_id=?4, place_note=?5, kind=?6, online=?7, note=?8,
                            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                     WHERE id = ?1",
                    params![id, s.date, s.date_prec, s.place_id, s.place_note, s.kind, s.online as i64, s.note],
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
                "INSERT INTO session_person (session_id, person_id, name_free, role, ord) VALUES (?1,?2,?3,?4,?5)",
                params![id, p.person_id, p.name_free, p.role, i as i64],
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
    match s.kind.as_str() {
        "keiko" | "koshukai" | "honban" => {}
        k => return Err(Error::Other(format!("bad session kind {k}"))),
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
            ("place.yasu-an", "place", "八洲庵"),
            ("person.noriko", "person", "則子先生"),
            ("person.izumi", "person", "出水さん"),
            ("temae.shin-no-gyo", "temae", "真之行台子"),
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

        // 10.05.2026 八洲庵 講習会 / 真乃行台子 出水さん 則子先生 客Ken / 真乃行台子 Ken 則子先生
        let s = Session {
            id: None,
            date: Some("2026-05-10".into()),
            date_prec: "day".into(),
            place_id: Some("place.yasu-an".into()),
            place_note: None,
            kind: "koshukai".into(),
            online: true,
            note: None,
            persons: vec![SessionPerson {
                person_id: Some("person.noriko".into()),
                name_free: None,
                role: "teacher".into(),
            }],
            temae: vec![
                Temae {
                    temae_id: Some("temae.shin-no-gyo".into()),
                    host_id: Some("person.izumi".into()),
                    teacher_id: Some("person.noriko".into()),
                    my_role: Some("kyaku".into()),
                    hearth: Some("furo".into()),
                    tea: Some("koicha".into()),
                    ..Default::default()
                },
                Temae {
                    temae_id: Some("temae.shin-no-gyo".into()),
                    host_id: Some(me.clone()),
                    my_role: Some("teishu".into()),
                    hearth: Some("furo".into()),
                    persons: vec![TemaePerson {
                        person_id: Some("person.izumi".into()),
                        name_free: None,
                        role: "kyaku".into(),
                        seat: Some(1),
                        is_last: false,
                        role_note: None,
                    }],
                    ..Default::default()
                },
            ],
        };
        let id = db.session_save(&s).unwrap();
        let back = db.session_get(id).unwrap().unwrap();
        assert_eq!(back.kind, "koshukai");
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
    fn unknown_date_allowed() {
        let db = Db::open_in_memory().unwrap();
        let s = Session { date_prec: "unknown".into(), ..Default::default() };
        db.session_save(&s).unwrap();
        let s = Session { date_prec: "day".into(), ..Default::default() };
        assert!(db.session_save(&s).is_err());
    }
}
