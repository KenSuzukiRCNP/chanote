//! Recency-weighted usage ranking for pick lists. Each use of an id contributes
//! `exp(-age_days / HALF_LIFE_DAYS * ln 2)` so a person you practised with weekly last
//! month outranks someone you met often three years ago. Rows never used get score 0
//! and keep their list order.

use crate::{Db, Result};
use serde::{Deserialize, Serialize};

pub const HALF_LIFE_DAYS: f64 = 365.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankRow {
    pub id: String,
    pub score: f64,
    pub n: i64,
    pub last_date: Option<String>,
}

impl Db {
    /// Ranking for one vocab kind: person | place | temae | tana | tool.
    pub fn vocab_ranking(&self, kind: &str) -> Result<Vec<RankRow>> {
        // Every (id, date) usage, unioned over the columns that can hold this kind.
        let uses = match kind {
            "person" => "SELECT t.host_id AS id, s.date FROM temae t JOIN session s ON s.id = t.session_id WHERE t.host_id IS NOT NULL
                UNION ALL SELECT t.teacher_id, s.date FROM temae t JOIN session s ON s.id = t.session_id WHERE t.teacher_id IS NOT NULL
                UNION ALL SELECT sp.person_id, s.date FROM session_person sp JOIN session s ON s.id = sp.session_id WHERE sp.person_id IS NOT NULL
                UNION ALL SELECT tp.person_id, s.date FROM temae_person tp JOIN temae t ON t.id = tp.temae_id JOIN session s ON s.id = t.session_id WHERE tp.person_id IS NOT NULL",
            "place" => "SELECT s.place_id AS id, s.date FROM session s WHERE s.place_id IS NOT NULL",
            "temae" => "SELECT COALESCE(t.temae_alias_id, t.temae_id) AS id, s.date FROM temae t JOIN session s ON s.id = t.session_id WHERE t.temae_id IS NOT NULL",
            "tana" => "SELECT t.tana_id AS id, s.date FROM temae t JOIN session s ON s.id = t.session_id WHERE t.tana_id IS NOT NULL AND t.tana_id NOT IN ('-','?')",
            "tool" => "SELECT tt.tool_id AS id, s.date FROM temae_tool tt JOIN temae t ON t.id = tt.temae_id JOIN session s ON s.id = t.session_id WHERE tt.tool_id IS NOT NULL",
            other => return Err(crate::Error::Other(format!("no ranking for kind {other}"))),
        };
        // (bundled SQLite has no exp(); do the weighting in Rust)
        let sql = format!(
            "SELECT id, date, julianday('now') - julianday(date) AS age FROM ({uses})"
        );
        let conn = self.conn();
        let mut st = conn.prepare(&sql)?;
        let mut acc: std::collections::HashMap<String, RankRow> = std::collections::HashMap::new();
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<f64>>(2)?))
        })?;
        for row in rows {
            let (id, date, age) = row?;
            let w = match age {
                Some(a) => (-(a.max(0.0)) / HALF_LIFE_DAYS * std::f64::consts::LN_2).exp(),
                None => 0.25,
            };
            let e = acc.entry(id.clone()).or_insert(RankRow { id, score: 0.0, n: 0, last_date: None });
            e.score += w;
            e.n += 1;
            if let Some(d) = date {
                if e.last_date.as_deref().map(|l| d.as_str() > l).unwrap_or(true) {
                    e.last_date = Some(d);
                }
            }
        }
        let mut out: Vec<RankRow> = acc.into_values().collect();
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.last_date.cmp(&a.last_date)));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Session, Temae, Vocab};

    fn person(db: &Db, id: &str) {
        db.vocab_upsert(&Vocab {
            id: id.into(), kind: "person".into(), ja: id.into(), romaji: None, en: None,
            parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None,
        }).unwrap();
    }
    fn sess(db: &Db, date: &str, host: &str) {
        db.session_save(&Session {
            date: Some(date.into()),
            temae: vec![Temae { host_id: Some(host.into()), ..Default::default() }],
            ..Default::default()
        }).unwrap();
    }

    #[test]
    fn recent_beats_frequent_but_old() {
        let db = Db::open_in_memory().unwrap();
        person(&db, "person.old");
        person(&db, "person.new");
        // old friend: 6 sessions, 4 years ago
        for m in 1..=6 { sess(&db, &format!("2022-0{m}-01"), "person.old"); }
        // new friend: 2 sessions, last month
        sess(&db, "2026-08-20", "person.new");
        sess(&db, "2026-09-05", "person.new");
        let r = db.vocab_ranking("person").unwrap();
        assert_eq!(r[0].id, "person.new");
        assert_eq!(r[1].id, "person.old");
        assert_eq!(r[1].n, 6);
        assert!(r[0].score > r[1].score);
        assert!(db.vocab_ranking("hearth").is_err());
    }
}
