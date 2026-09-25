//! School packs: YAML files describing a school's vocabulary (temae, tana, hearth ...).
//! See packs/urasenke.yaml for the format.

use crate::model::{AxisValues, TemaeMeta, Vocab};
use crate::{Db, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub version: String,
    pub name: LocalizedName,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub temae: Vec<PackTemae>,
    #[serde(default)]
    pub tana: Vec<PackItem>,
    #[serde(default)]
    pub hearth: Vec<PackItem>,
    /// Utensil categories (茶碗, 薄器, ...). Users add tools under these.
    #[serde(default)]
    pub tool_categories: Vec<PackItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedName {
    pub ja: String,
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub ja: String,
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackItem {
    pub id: String,
    pub ja: String,
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackTemae {
    pub id: String,
    pub ja: String,
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub defaults: Option<AxisValues>,
    #[serde(default)]
    pub alias_of: Option<String>,
    #[serde(default)]
    pub modifiers: Option<AxisValues>,
}

impl Pack {
    pub fn from_yaml(s: &str) -> Result<Pack> {
        let p: Pack = serde_yaml_ng::from_str(s)?;
        p.validate()?;
        Ok(p)
    }

    fn validate(&self) -> Result<()> {
        let temae_ids: std::collections::HashSet<&str> =
            self.temae.iter().map(|t| t.id.as_str()).collect();
        let cat_ids: std::collections::HashSet<&str> =
            self.categories.iter().map(|c| c.id.as_str()).collect();
        for t in &self.temae {
            if let Some(a) = &t.alias_of {
                if !temae_ids.contains(a.as_str()) {
                    return Err(Error::Pack(format!("{}: alias_of {} not in pack", t.id, a)));
                }
            }
            if let Some(c) = &t.category {
                if !cat_ids.contains(c.as_str()) {
                    return Err(Error::Pack(format!("{}: category {} not defined", t.id, c)));
                }
            }
        }
        Ok(())
    }

    /// Flatten into vocab rows. Ids are prefixed by kind ("temae.", "tana.", ...).
    pub fn to_vocab(&self) -> Vec<Vocab> {
        let origin = format!("pack:{}", self.id);
        let mut out = Vec::new();
        let mk = |id: String, kind: &str, ja: &str, romaji: &Option<String>, en: &Option<String>,
                  parent: Option<String>, sort: i64, meta: Option<serde_json::Value>| Vocab {
            id,
            kind: kind.into(),
            ja: ja.into(),
            romaji: romaji.clone(),
            en: en.clone(),
            parent_id: parent,
            sort: Some(sort),
            origin: origin.clone(),
            archived: false,
            meta,
        };

        out.push(mk(
            format!("school.{}", self.id), "school", &self.name.ja, &self.name.romaji,
            &self.name.en, None, 0, Some(json!({"version": self.version})),
        ));
        // categories live in the temae kind as group headers (parent for temae rows)
        for (i, c) in self.categories.iter().enumerate() {
            out.push(mk(
                format!("temae-cat.{}", c.id), "temae-cat", &c.ja, &c.romaji, &c.en,
                c.parent.as_ref().map(|p| format!("temae-cat.{p}")), i as i64, None,
            ));
        }
        for (i, t) in self.temae.iter().enumerate() {
            let meta = TemaeMeta {
                category: t.category.clone(),
                defaults: t.defaults.clone(),
                alias_of: t.alias_of.as_ref().map(|a| format!("temae.{a}")),
                modifiers: t.modifiers.clone(),
            };
            out.push(mk(
                format!("temae.{}", t.id), "temae", &t.ja, &t.romaji, &t.en,
                t.category.as_ref().map(|c| format!("temae-cat.{c}")), i as i64,
                Some(serde_json::to_value(meta).unwrap()),
            ));
        }
        for (i, x) in self.tana.iter().enumerate() {
            out.push(mk(
                format!("tana.{}", x.id), "tana", &x.ja, &x.romaji, &x.en,
                x.parent.as_ref().map(|p| format!("tana.{p}")), i as i64, None,
            ));
        }
        for (i, x) in self.hearth.iter().enumerate() {
            out.push(mk(
                format!("hearth.{}", x.id), "hearth", &x.ja, &x.romaji, &x.en, None, i as i64, None,
            ));
        }
        for (i, x) in self.tool_categories.iter().enumerate() {
            out.push(mk(
                format!("tool-cat.{}", x.id), "tool-cat", &x.ja, &x.romaji, &x.en, None, i as i64, None,
            ));
        }
        out
    }
}

impl Db {
    /// Install or upgrade a pack. Only rows with origin 'pack:<id>' are touched;
    /// user-added rows are left alone. Returns true if anything was written.
    pub fn pack_install(&self, pack: &Pack) -> Result<bool> {
        let installed: Option<String> = {
            let conn = self.conn();
            conn.query_row(
                "SELECT pack_version FROM pack_installed WHERE pack_id = ?1",
                [&pack.id],
                |r| r.get(0),
            )
            .ok()
        };
        if installed.as_deref() == Some(pack.version.as_str()) {
            return Ok(false);
        }
        // order: parents (school, categories, tana parents) before children — to_vocab already
        // emits categories before temae; tana parents must precede children in the YAML.
        for v in pack.to_vocab() {
            self.vocab_upsert(&v)?;
        }
        self.conn().execute(
            "INSERT INTO pack_installed (pack_id, pack_version) VALUES (?1, ?2)
             ON CONFLICT(pack_id) DO UPDATE SET pack_version = excluded.pack_version,
               installed_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
            [&pack.id, &pack.version],
        )?;
        Ok(true)
    }
}

/// The Urasenke pack shipped with the app.
pub const URASENKE_YAML: &str = include_str!("../../../packs/urasenke.yaml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urasenke_pack_parses_and_installs() {
        let pack = Pack::from_yaml(URASENKE_YAML).unwrap();
        assert_eq!(pack.id, "urasenke");
        let db = Db::open_in_memory().unwrap();
        assert!(db.pack_install(&pack).unwrap());
        assert!(!db.pack_install(&pack).unwrap()); // same version → no-op

        let temae = db.vocab_list("temae", false).unwrap();
        assert!(temae.len() > 50, "got {}", temae.len());
        // every alias resolves
        for t in &temae {
            let r = db.resolve_temae(&t.id).unwrap();
            assert!(db.vocab_get(&r.temae_id).unwrap().is_some());
        }
        // spot checks from the spec
        let r = db.resolve_temae("temae.karamono").unwrap();
        assert_eq!(r.axes.tea.as_deref(), Some("koicha"));
        let r = db.resolve_temae("temae.kagetsu.koicha").unwrap();
        assert_eq!(r.temae_id, "temae.kagetsu");
        assert_eq!(r.axes.tea.as_deref(), Some("koicha"));
        let r = db.resolve_temae("temae.kinin-kiyotsugu").unwrap();
        assert_eq!(r.axes.kinin.as_deref(), Some("kiyotsugu"));
        let r = db.resolve_temae("temae.shin-no-gyo").unwrap();
        assert_eq!(r.axes.tana.as_deref(), Some("tana.daisu.shin"));
        assert!(db.vocab_get("tana.daisu.shin").unwrap().is_some());
        assert!(db.vocab_list("tool-cat", false).unwrap().len() >= 7);
    }

    #[test]
    fn user_rows_survive_pack_upgrade() {
        let pack = Pack::from_yaml(URASENKE_YAML).unwrap();
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&pack).unwrap();
        db.vocab_upsert(&Vocab {
            id: "temae.my-ikea".into(),
            kind: "temae".into(),
            ja: "IKEA棚点前".into(),
            romaji: None,
            en: None,
            parent_id: None,
            sort: None,
            origin: "user".into(),
            archived: false,
            meta: None,
        })
        .unwrap();
        let mut newer = pack.clone();
        newer.version = "999".into();
        assert!(db.pack_install(&newer).unwrap());
        assert!(db.vocab_get("temae.my-ikea").unwrap().is_some());
    }
}
