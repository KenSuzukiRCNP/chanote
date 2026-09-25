//! Aggregations for the statistics screens. Everything is computed in SQL and returned
//! as small, pre-grouped rows; the frontend does the final layout (coverage table,
//! "not done recently" ranking).

use crate::{Db, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// One cell of the coverage cube: (temae, alias, hearth, my-role-group).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverageRow {
    /// Main temae id (alias already resolved), or None for free-text temae.
    pub temae_id: Option<String>,
    /// Alias the user picked, if any (e.g. temae.kagetsu.koicha).
    pub alias_id: Option<String>,
    /// Raw hearth value: ro | furo | binkake | ... | "?" | None.
    pub hearth: Option<String>,
    /// "host" (my_role = teishu) | "other" (any other participation) | "absent" (null or '-').
    pub role: String,
    pub n: i64,
    /// Count within the last `recent_days` (parameter of the query).
    pub n_recent: i64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsFilter {
    /// Only sessions with date >= since (YYYY-MM-DD).
    pub since: Option<String>,
    /// Only sessions with date <= until.
    pub until: Option<String>,
    /// Window for n_recent, in days (default 365).
    pub recent_days: Option<i64>,
    /// Restrict to one place (session.place_id) if set.
    pub place_id: Option<String>,
}

impl Db {
    pub fn stats_coverage(&self, f: &StatsFilter) -> Result<Vec<CoverageRow>> {
        let conn = self.conn();
        let recent = f.recent_days.unwrap_or(365);
        let mut st = conn.prepare(
            "SELECT t.temae_id, t.temae_alias_id, t.hearth,
                    CASE
                      WHEN t.my_role = 'teishu' THEN 'host'
                      WHEN t.my_role IS NULL OR t.my_role = '-' THEN 'absent'
                      ELSE 'other'
                    END AS role_group,
                    COUNT(*) AS n,
                    SUM(CASE WHEN s.date IS NOT NULL
                              AND julianday('now') - julianday(s.date) <= ?1 THEN 1 ELSE 0 END) AS n_recent,
                    MIN(s.date) AS first_date,
                    MAX(s.date) AS last_date
             FROM temae t
             JOIN session s ON s.id = t.session_id
             WHERE (?2 IS NULL OR s.date >= ?2)
               AND (?3 IS NULL OR s.date <= ?3)
               AND (?4 IS NULL OR s.place_id = ?4)
             GROUP BY t.temae_id, t.temae_alias_id, t.hearth, role_group
             ORDER BY t.temae_id, t.temae_alias_id, t.hearth, role_group",
        )?;
        let rows = st.query_map(params![recent, f.since, f.until, f.place_id], |r| {
            Ok(CoverageRow {
                temae_id: r.get(0)?,
                alias_id: r.get(1)?,
                hearth: r.get(2)?,
                role: r.get(3)?,
                n: r.get(4)?,
                n_recent: r.get(5)?,
                first_date: r.get(6)?,
                last_date: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Simple totals for the header of the stats page.
    pub fn stats_totals(&self, f: &StatsFilter) -> Result<StatsTotals> {
        let conn = self.conn();
        let row = conn.query_row(
            "SELECT
               (SELECT COUNT(*) FROM session s
                 WHERE (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2)
                   AND (?3 IS NULL OR s.place_id = ?3)),
               (SELECT COUNT(*) FROM temae t JOIN session s ON s.id = t.session_id
                 WHERE t.my_role = 'teishu'
                   AND (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2)
                   AND (?3 IS NULL OR s.place_id = ?3)),
               (SELECT COUNT(*) FROM temae t JOIN session s ON s.id = t.session_id
                 WHERE t.my_role IS NOT NULL AND t.my_role NOT IN ('teishu', '-')
                   AND (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2)
                   AND (?3 IS NULL OR s.place_id = ?3)),
               (SELECT MIN(date) FROM session), (SELECT MAX(date) FROM session)",
            params![f.since, f.until, f.place_id],
            |r| {
                Ok(StatsTotals {
                    sessions: r.get(0)?,
                    temae_as_host: r.get(1)?,
                    temae_as_other: r.get(2)?,
                    first_date: r.get(3)?,
                    last_date: r.get(4)?,
                })
            },
        )?;
        Ok(row)
    }
}


/// One month of activity, split by hearth group and my-role group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MonthlyRow {
    /// "YYYY-MM"
    pub month: String,
    /// ro | furo | other (collapsed: ro/dairo/tsurigama/sukigi → ro, furo/nakaoki → furo)
    pub hearth: String,
    /// host | other | absent
    pub role: String,
    pub n_temae: i64,
    /// distinct sessions contributing to this cell
    pub n_sessions: i64,
}

const HEARTH_GROUP_SQL: &str = "CASE
    WHEN t.hearth IN ('ro','dairo','tsurigama','sukigi') THEN 'ro'
    WHEN t.hearth IN ('furo','nakaoki') THEN 'furo'
    ELSE 'other' END";

const ROLE_GROUP_SQL: &str = "CASE
    WHEN t.my_role = 'teishu' THEN 'host'
    WHEN t.my_role IS NULL OR t.my_role = '-' THEN 'absent'
    ELSE 'other' END";

/// Axes available for the cross table. Each maps to a SQL expression over
/// `temae t JOIN session s`.
pub const CROSS_AXES: &[(&str, &str)] = &[
    ("temae", "t.temae_id"),
    ("alias", "COALESCE(t.temae_alias_id, t.temae_id)"),
    ("hearth", "t.hearth"),
    ("hearth_group", HEARTH_GROUP_SQL),
    ("tana", "t.tana_id"),
    ("tea", "t.tea"),
    ("katte", "t.katte"),
    ("room", "t.room"),
    ("kinin", "t.kinin"),
    ("host", "t.host_id"),
    ("teacher", "COALESCE(t.teacher_id, (SELECT sp.person_id FROM session_person sp WHERE sp.session_id = s.id AND sp.role = 'teacher' ORDER BY sp.ord LIMIT 1))"),
    ("place", "s.place_id"),
    ("my_role", "t.my_role"),
    ("year", "substr(s.date, 1, 4)"),
    ("session_kind", "s.kind"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrossCell {
    pub row: Option<String>,
    pub col: Option<String>,
    pub n: i64,
    pub last_date: Option<String>,
}

impl Db {
    pub fn stats_monthly(&self, f: &StatsFilter) -> Result<Vec<MonthlyRow>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT substr(s.date, 1, 7) AS month, {HEARTH_GROUP_SQL} AS hg, {ROLE_GROUP_SQL} AS rg,
                    COUNT(*) AS n_temae, COUNT(DISTINCT s.id) AS n_sessions
             FROM temae t JOIN session s ON s.id = t.session_id
             WHERE s.date IS NOT NULL
               AND (?1 IS NULL OR s.date >= ?1)
               AND (?2 IS NULL OR s.date <= ?2)
               AND (?3 IS NULL OR s.place_id = ?3)
             GROUP BY month, hg, rg
             ORDER BY month, hg, rg"
        );
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map(params![f.since, f.until, f.place_id], |r| {
            Ok(MonthlyRow {
                month: r.get(0)?,
                hearth: r.get(1)?,
                role: r.get(2)?,
                n_temae: r.get(3)?,
                n_sessions: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Cross table over two axes from `CROSS_AXES`. `host_only` restricts to my_role = teishu.
    pub fn stats_cross(&self, row_axis: &str, col_axis: &str, host_only: bool, f: &StatsFilter) -> Result<Vec<CrossCell>> {
        let expr = |name: &str| -> Result<&'static str> {
            CROSS_AXES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, e)| *e)
                .ok_or_else(|| crate::Error::Other(format!("unknown axis {name}")))
        };
        let (re, ce) = (expr(row_axis)?, expr(col_axis)?);
        let conn = self.conn();
        let sql = format!(
            "SELECT {re} AS r, {ce} AS c, COUNT(*) AS n, MAX(s.date) AS last_date
             FROM temae t JOIN session s ON s.id = t.session_id
             WHERE (?1 IS NULL OR s.date >= ?1)
               AND (?2 IS NULL OR s.date <= ?2)
               AND (?3 IS NULL OR s.place_id = ?3)
               AND (?4 = 0 OR t.my_role = 'teishu')
             GROUP BY r, c
             ORDER BY r, c"
        );
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map(params![f.since, f.until, f.place_id, host_only as i64], |r| {
            Ok(CrossCell { row: r.get(0)?, col: r.get(1)?, n: r.get(2)?, last_date: r.get(3)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// One row of a "by person / by place" table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ByRow {
    /// vocab id (or free-text name prefixed with "free:")
    pub key: String,
    /// sessions involved
    pub n_sessions: i64,
    /// my temae as host in those sessions (for teacher/place: under that teacher / at that place)
    pub n_host: i64,
    /// temae where I was present but not host
    pub n_other: i64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

impl Db {
    /// dimension: teacher | place | host (whose temae I attended) | with (co-participants of my temae)
    pub fn stats_by(&self, dimension: &str, f: &StatsFilter) -> Result<Vec<ByRow>> {
        let sql = match dimension {
            "teacher" => "SELECT k, COUNT(DISTINCT sid), SUM(host), SUM(oth), MIN(d), MAX(d) FROM (
                SELECT COALESCE(t.teacher_id, (SELECT sp.person_id FROM session_person sp WHERE sp.session_id = s.id AND sp.role='teacher' ORDER BY sp.ord LIMIT 1)) AS k,
                       s.id AS sid, s.date AS d,
                       CASE WHEN t.my_role='teishu' THEN 1 ELSE 0 END AS host,
                       CASE WHEN t.my_role IS NOT NULL AND t.my_role NOT IN ('teishu','-') THEN 1 ELSE 0 END AS oth
                FROM temae t JOIN session s ON s.id = t.session_id
                WHERE (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2) AND (?3 IS NULL OR s.place_id = ?3)
              ) WHERE k IS NOT NULL GROUP BY k ORDER BY 2 DESC",
            "place" => "SELECT k, COUNT(DISTINCT sid), SUM(host), SUM(oth), MIN(d), MAX(d) FROM (
                SELECT s.place_id AS k, s.id AS sid, s.date AS d,
                       CASE WHEN t.my_role='teishu' THEN 1 ELSE 0 END AS host,
                       CASE WHEN t.my_role IS NOT NULL AND t.my_role NOT IN ('teishu','-') THEN 1 ELSE 0 END AS oth
                FROM session s LEFT JOIN temae t ON t.session_id = s.id
                WHERE (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2) AND (?3 IS NULL OR s.place_id = ?3)
              ) WHERE k IS NOT NULL GROUP BY k ORDER BY 2 DESC",
            // whose temae did I attend (host ≠ me)
            "host" => "SELECT k, COUNT(DISTINCT sid), 0, SUM(oth), MIN(d), MAX(d) FROM (
                SELECT COALESCE(t.host_id, 'free:' || t.host_free) AS k, s.id AS sid, s.date AS d,
                       CASE WHEN t.my_role IS NOT NULL AND t.my_role NOT IN ('teishu','-') THEN 1 ELSE 0 END AS oth
                FROM temae t JOIN session s ON s.id = t.session_id
                WHERE t.my_role IS NOT NULL AND t.my_role <> 'teishu'
                  AND (t.host_id IS NOT NULL OR t.host_free IS NOT NULL)
                  AND (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2) AND (?3 IS NULL OR s.place_id = ?3)
              ) WHERE k IS NOT NULL GROUP BY k ORDER BY 4 DESC",
            // people who were guests etc. when I was host
            "with" => "SELECT k, COUNT(DISTINCT sid), SUM(host), 0, MIN(d), MAX(d) FROM (
                SELECT COALESCE(tp.person_id, 'free:' || tp.name_free) AS k, s.id AS sid, s.date AS d, 1 AS host
                FROM temae_person tp JOIN temae t ON t.id = tp.temae_id JOIN session s ON s.id = t.session_id
                WHERE t.my_role = 'teishu'
                  AND (?1 IS NULL OR s.date >= ?1) AND (?2 IS NULL OR s.date <= ?2) AND (?3 IS NULL OR s.place_id = ?3)
              ) WHERE k IS NOT NULL GROUP BY k ORDER BY 3 DESC",
            other => return Err(crate::Error::Other(format!("unknown dimension {other}"))),
        };
        let conn = self.conn();
        let mut st = conn.prepare(sql)?;
        let rows = st.query_map(params![f.since, f.until, f.place_id], |r| {
            Ok(ByRow {
                key: r.get(0)?,
                n_sessions: r.get(1)?,
                n_host: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                n_other: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                first_date: r.get(4)?,
                last_date: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsTotals {
    pub sessions: i64,
    pub temae_as_host: i64,
    pub temae_as_other: i64,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Session, Temae};

    fn session(date: &str, temae: Vec<Temae>) -> Session {
        Session { date: Some(date.into()), temae, ..Default::default() }
    }
    fn tm(id: &str, hearth: &str, role: Option<&str>) -> Temae {
        Temae {
            temae_id: Some(id.into()),
            hearth: Some(hearth.into()),
            my_role: role.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn coverage_groups_by_role_and_hearth() {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&crate::pack::Pack::from_yaml(crate::pack::URASENKE_YAML).unwrap()).unwrap();
        db.session_save(&session("2025-02-01", vec![tm("temae.karamono", "ro", Some("teishu"))])).unwrap();
        db.session_save(&session("2025-07-01", vec![
            tm("temae.karamono", "furo", Some("teishu")),
            tm("temae.karamono", "furo", Some("kyaku")),
            tm("temae.kagetsu", "furo", None),
        ])).unwrap();

        let rows = db.stats_coverage(&StatsFilter::default()).unwrap();
        let find = |t: &str, h: &str, r: &str| {
            rows.iter().find(|x| x.temae_id.as_deref() == Some(t) && x.hearth.as_deref() == Some(h) && x.role == r).cloned()
        };
        assert_eq!(find("temae.karamono", "ro", "host").unwrap().n, 1);
        assert_eq!(find("temae.karamono", "furo", "host").unwrap().n, 1);
        assert_eq!(find("temae.karamono", "furo", "other").unwrap().n, 1);
        assert_eq!(find("temae.kagetsu", "furo", "absent").unwrap().n, 1);
        assert_eq!(find("temae.karamono", "furo", "host").unwrap().last_date.as_deref(), Some("2025-07-01"));

        // since filter
        let rows = db.stats_coverage(&StatsFilter { since: Some("2025-06-01".into()), ..Default::default() }).unwrap();
        assert!(rows.iter().all(|x| x.hearth.as_deref() == Some("furo")));

        let t = db.stats_totals(&StatsFilter::default()).unwrap();
        assert_eq!(t.sessions, 2);
        assert_eq!(t.temae_as_host, 2);
        assert_eq!(t.temae_as_other, 1);

        // monthly
        let m = db.stats_monthly(&StatsFilter::default()).unwrap();
        let july_host = m.iter().find(|x| x.month == "2025-07" && x.hearth == "furo" && x.role == "host").unwrap();
        assert_eq!(july_host.n_temae, 1);
        assert_eq!(july_host.n_sessions, 1);
        assert!(m.iter().any(|x| x.month == "2025-02" && x.hearth == "ro"));

        // cross: temae × hearth_group, all roles
        let c = db.stats_cross("temae", "hearth_group", false, &StatsFilter::default()).unwrap();
        let cell = c.iter().find(|x| x.row.as_deref() == Some("temae.karamono") && x.col.as_deref() == Some("furo")).unwrap();
        assert_eq!(cell.n, 2);
        // host only
        let c = db.stats_cross("temae", "hearth_group", true, &StatsFilter::default()).unwrap();
        let cell = c.iter().find(|x| x.row.as_deref() == Some("temae.karamono") && x.col.as_deref() == Some("furo")).unwrap();
        assert_eq!(cell.n, 1);
        assert!(db.stats_cross("nope", "tea", false, &StatsFilter::default()).is_err());

        // by place / teacher / host
        db.vocab_upsert(&crate::model::Vocab { id: "place.a".into(), kind: "place".into(), ja: "A".into(), romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None }).unwrap();
        db.vocab_upsert(&crate::model::Vocab { id: "person.t".into(), kind: "person".into(), ja: "T".into(), romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None }).unwrap();
        db.session_save(&Session {
            date: Some("2026-01-01".into()), place_id: Some("place.a".into()),
            persons: vec![crate::model::SessionPerson { person_id: Some("person.t".into()), name_free: None, role: "teacher".into() }],
            temae: vec![
                Temae { temae_id: Some("temae.karamono".into()), my_role: Some("teishu".into()), ..Default::default() },
                Temae { temae_id: Some("temae.karamono".into()), my_role: Some("kyaku".into()), host_id: Some("person.t".into()), ..Default::default() },
            ],
            ..Default::default()
        }).unwrap();
        let by = db.stats_by("place", &StatsFilter::default()).unwrap();
        let a = by.iter().find(|x| x.key == "place.a").unwrap();
        assert_eq!((a.n_sessions, a.n_host, a.n_other), (1, 1, 1));
        let by = db.stats_by("teacher", &StatsFilter::default()).unwrap();
        assert_eq!(by[0].key, "person.t");
        assert_eq!(by[0].n_host, 1);
        let by = db.stats_by("host", &StatsFilter::default()).unwrap();
        assert_eq!(by[0].key, "person.t");
        assert_eq!(by[0].n_other, 1);
        assert!(db.stats_by("x", &StatsFilter::default()).is_err());
    }
}
