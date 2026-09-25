use crate::model::{AxisValues, ResolvedTemae, TemaeMeta, Vocab};
use crate::{Db, Error, Result};
use rusqlite::{params, OptionalExtension, Row};

fn row_to_vocab(r: &Row) -> rusqlite::Result<Vocab> {
    let meta: Option<String> = r.get("meta")?;
    Ok(Vocab {
        id: r.get("id")?,
        kind: r.get("kind")?,
        ja: r.get("ja")?,
        romaji: r.get("romaji")?,
        en: r.get("en")?,
        parent_id: r.get("parent_id")?,
        sort: r.get("sort")?,
        origin: r.get("origin")?,
        archived: r.get::<_, i64>("archived")? != 0,
        meta: meta.and_then(|s| serde_json::from_str(&s).ok()),
    })
}

const COLS: &str = "id, kind, ja, romaji, en, parent_id, sort, origin, archived, meta";

impl Db {
    pub fn vocab_list(&self, kind: &str, include_archived: bool) -> Result<Vec<Vocab>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {COLS} FROM vocab WHERE kind = ?1 {} ORDER BY sort, ja",
            if include_archived { "" } else { "AND archived = 0" }
        );
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map([kind], row_to_vocab)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn vocab_get(&self, id: &str) -> Result<Option<Vocab>> {
        let conn = self.conn();
        let sql = format!("SELECT {COLS} FROM vocab WHERE id = ?1");
        Ok(conn.query_row(&sql, [id], row_to_vocab).optional()?)
    }

    /// Insert or update one vocab row.
    pub fn vocab_upsert(&self, v: &Vocab) -> Result<()> {
        let meta = match &v.meta {
            Some(m) => Some(serde_json::to_string(m)?),
            None => None,
        };
        self.conn().execute(
            "INSERT INTO vocab (id, kind, ja, romaji, en, parent_id, sort, origin, archived, meta)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
               kind = excluded.kind, ja = excluded.ja, romaji = excluded.romaji,
               en = excluded.en, parent_id = excluded.parent_id, sort = excluded.sort,
               origin = excluded.origin, archived = excluded.archived, meta = excluded.meta",
            params![
                v.id, v.kind, v.ja, v.romaji, v.en, v.parent_id, v.sort, v.origin,
                v.archived as i64, meta
            ],
        )?;
        Ok(())
    }

    pub fn vocab_archive(&self, id: &str, archived: bool) -> Result<()> {
        self.conn().execute(
            "UPDATE vocab SET archived = ?2 WHERE id = ?1",
            params![id, archived as i64],
        )?;
        Ok(())
    }

    /// How many records reference this vocab id (any column that can hold it).
    pub fn vocab_usage(&self, id: &str) -> Result<i64> {
        let conn = self.conn();
        let n: i64 = conn.query_row(
            "SELECT
               (SELECT COUNT(*) FROM session WHERE place_id = ?1)
             + (SELECT COUNT(*) FROM session_person WHERE person_id = ?1)
             + (SELECT COUNT(*) FROM temae WHERE temae_id = ?1 OR temae_alias_id = ?1 OR host_id = ?1
                                            OR teacher_id = ?1 OR tana_id = ?1)
             + (SELECT COUNT(*) FROM temae_person WHERE person_id = ?1)
             + (SELECT COUNT(*) FROM temae_tool WHERE tool_id = ?1)
             + (SELECT COUNT(*) FROM vocab WHERE parent_id = ?1)
             + (SELECT COUNT(*) FROM vocab WHERE json_extract(meta, '$.alias_of') = ?1)",
            [id],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    /// Delete a user-added vocab row. Refuses pack rows, the self person, and rows still
    /// referenced by records (archive those instead).
    pub fn vocab_delete(&self, id: &str) -> Result<()> {
        let v = self.vocab_get(id)?.ok_or_else(|| Error::Other(format!("unknown {id}")))?;
        if v.origin != "user" {
            return Err(Error::Other("pack entries cannot be deleted; archive instead".into()));
        }
        if v.meta.as_ref().and_then(|m| m.get("is_self")).and_then(|x| x.as_bool()) == Some(true) {
            return Err(Error::Other("the self person cannot be deleted".into()));
        }
        let n = self.vocab_usage(id)?;
        if n > 0 {
            return Err(Error::InUse(n));
        }
        self.conn().execute("DELETE FROM vocab WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Ensure the single "self" person exists; returns its id.
    pub fn self_person_id(&self) -> Result<String> {
        let conn = self.conn();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM vocab WHERE kind = 'person' AND json_extract(meta, '$.is_self') = 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        conn.execute(
            "INSERT INTO vocab (id, kind, ja, romaji, en, sort, origin, meta)
             VALUES ('person.self', 'person', '自分', 'jibun', 'myself', 0, 'user', '{\"is_self\":true}')",
            [],
        )?;
        Ok("person.self".into())
    }

    /// Resolve a chosen temae id (which may be an alias) into the stored main id
    /// plus prefilled axis values (defaults of the main temae overlaid with alias modifiers).
    pub fn resolve_temae(&self, chosen_id: &str) -> Result<ResolvedTemae> {
        let chosen = self
            .vocab_get(chosen_id)?
            .ok_or_else(|| Error::Other(format!("unknown temae {chosen_id}")))?;
        let chosen_meta: TemaeMeta = chosen
            .meta
            .clone()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();

        match chosen_meta.alias_of {
            None => Ok(ResolvedTemae {
                temae_id: chosen.id,
                temae_alias_id: None,
                axes: chosen_meta.defaults.unwrap_or_default(),
            }),
            Some(main_id) => {
                let main = self
                    .vocab_get(&main_id)?
                    .ok_or_else(|| Error::Other(format!("alias target {main_id} missing")))?;
                let main_meta: TemaeMeta = main
                    .meta
                    .map(serde_json::from_value)
                    .transpose()?
                    .unwrap_or_default();
                let base = main_meta.defaults.unwrap_or_default();
                let axes = base.overlay(&chosen_meta.modifiers.unwrap_or_default());
                Ok(ResolvedTemae {
                    temae_id: main.id,
                    temae_alias_id: Some(chosen.id),
                    axes,
                })
            }
        }
    }

    /// Axis defaults for a main temae without alias handling (used by the UI when the
    /// user switches the main temae after an alias was expanded).
    pub fn temae_defaults(&self, id: &str) -> Result<AxisValues> {
        Ok(self.resolve_temae(id)?.axes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v(id: &str, kind: &str, ja: &str, meta: serde_json::Value) -> Vocab {
        Vocab {
            id: id.into(),
            kind: kind.into(),
            ja: ja.into(),
            romaji: None,
            en: None,
            parent_id: None,
            sort: None,
            origin: "user".into(),
            archived: false,
            meta: Some(meta),
        }
    }

    #[test]
    fn alias_resolution_overlays_defaults() {
        let db = Db::open_in_memory().unwrap();
        db.vocab_upsert(&v(
            "temae.kagetsu",
            "temae",
            "花月之式",
            json!({"category":"shichijishiki","defaults":{"tea":"usucha","tana":"-"}}),
        ))
        .unwrap();
        db.vocab_upsert(&v(
            "temae.kagetsu.koicha",
            "temae",
            "濃茶付花月",
            json!({"alias_of":"temae.kagetsu","modifiers":{"tea":"koicha","tsukimono":["koicha"]}}),
        ))
        .unwrap();

        let r = db.resolve_temae("temae.kagetsu.koicha").unwrap();
        assert_eq!(r.temae_id, "temae.kagetsu");
        assert_eq!(r.temae_alias_id.as_deref(), Some("temae.kagetsu.koicha"));
        assert_eq!(r.axes.tea.as_deref(), Some("koicha"));
        assert_eq!(r.axes.tana.as_deref(), Some("-")); // inherited
        assert_eq!(r.axes.tsukimono, Some(vec!["koicha".to_string()]));

        let r = db.resolve_temae("temae.kagetsu").unwrap();
        assert_eq!(r.temae_alias_id, None);
        assert_eq!(r.axes.tea.as_deref(), Some("usucha"));
    }

    #[test]
    fn delete_guards() {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&crate::pack::Pack::from_yaml(crate::pack::URASENKE_YAML).unwrap()).unwrap();
        let me = db.self_person_id().unwrap();
        db.vocab_upsert(&v("person.x", "person", "Jasmin", json!({}))).unwrap();
        db.vocab_upsert(&v("person.y", "person", "Gernot", json!({}))).unwrap();
        // pack row → refused
        assert!(db.vocab_delete("temae.karamono").is_err());
        assert!(db.vocab_delete(&me).is_err());
        // used row → InUse
        db.session_save(&crate::model::Session {
            date: Some("2026-01-01".into()),
            temae: vec![crate::model::Temae { host_id: Some("person.x".into()), ..Default::default() }],
            ..Default::default()
        }).unwrap();
        assert!(matches!(db.vocab_delete("person.x"), Err(Error::InUse(1))));
        // unused → deleted
        db.vocab_delete("person.y").unwrap();
        assert!(db.vocab_get("person.y").unwrap().is_none());
    }

    #[test]
    fn self_person_is_created_once() {
        let db = Db::open_in_memory().unwrap();
        let a = db.self_person_id().unwrap();
        let b = db.self_person_id().unwrap();
        assert_eq!(a, b);
        assert_eq!(db.vocab_list("person", false).unwrap().len(), 1);
    }
}
