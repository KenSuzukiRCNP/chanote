//! Serde types shared between Rust and the frontend (mirrored in src/lib/types.ts).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vocab {
    pub id: String,
    pub kind: String,
    pub ja: String,
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort: Option<i64>,
    #[serde(default = "default_origin")]
    pub origin: String,
    #[serde(default)]
    pub archived: bool,
    /// Kind-specific attributes (see design doc §2.2). Stored as JSON text.
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

fn default_origin() -> String {
    "user".into()
}

/// Attributes a temae vocab entry may carry in `meta`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TemaeMeta {
    /// hira | konarai.mae8 | konarai.ato8 | sumi | ouyou | chabako | shikaden | okuden | shichijishiki
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Default axis values applied when this temae is chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<AxisValues>,
    /// If set, this entry is an alias: the record stores `alias_of` as temae_id
    /// and this id as temae_alias_id, and `modifiers` are applied on top of defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<AxisValues>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AxisValues {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hearth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub katte: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tea: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tana: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tsukimono: Option<Vec<String>>,
}

impl AxisValues {
    /// `other` wins where it is set.
    pub fn overlay(&self, other: &AxisValues) -> AxisValues {
        AxisValues {
            hearth: other.hearth.clone().or_else(|| self.hearth.clone()),
            katte: other.katte.clone().or_else(|| self.katte.clone()),
            room: other.room.clone().or_else(|| self.room.clone()),
            tea: other.tea.clone().or_else(|| self.tea.clone()),
            tana: other.tana.clone().or_else(|| self.tana.clone()),
            kinin: other.kinin.clone().or_else(|| self.kinin.clone()),
            tsukimono: other.tsukimono.clone().or_else(|| self.tsukimono.clone()),
        }
    }
}

/// Result of resolving a temae choice (possibly an alias) into stored ids + prefilled axes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedTemae {
    pub temae_id: String,
    pub temae_alias_id: Option<String>,
    pub axes: AxisValues,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    #[serde(default)]
    pub id: Option<i64>,
    pub date: Option<String>,
    #[serde(default = "default_date_prec")]
    pub date_prec: String,
    pub place_id: Option<String>,
    pub place_note: Option<String>,
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Joined remotely (place_id is still the group's place).
    #[serde(default)]
    pub online: bool,
    pub note: Option<String>,
    #[serde(default)]
    pub persons: Vec<SessionPerson>,
    #[serde(default)]
    pub temae: Vec<Temae>,
}

fn default_date_prec() -> String {
    "day".into()
}
fn default_kind() -> String {
    "keiko".into()
}

impl Default for Session {
    fn default() -> Self {
        Session {
            id: None,
            date: None,
            date_prec: default_date_prec(),
            place_id: None,
            place_note: None,
            kind: default_kind(),
            online: false,
            note: None,
            persons: vec![],
            temae: vec![],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionPerson {
    pub person_id: Option<String>,
    pub name_free: Option<String>,
    pub role: String, // teacher | guest
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Temae {
    #[serde(default)]
    pub id: Option<i64>,
    pub time: Option<String>,
    pub temae_id: Option<String>,
    pub temae_alias_id: Option<String>,
    pub temae_free: Option<String>,
    pub host_id: Option<String>,
    pub host_free: Option<String>,
    pub teacher_id: Option<String>,
    pub my_role: Option<String>,
    pub my_role_note: Option<String>,
    pub my_seat: Option<i64>,
    pub hearth: Option<String>,
    pub katte: Option<String>,
    pub room: Option<String>,
    pub tea: Option<String>,
    pub tana_id: Option<String>,
    pub tana_free: Option<String>,
    pub kinin: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub persons: Vec<TemaePerson>,
    #[serde(default)]
    pub tsukimono: Vec<String>,
    #[serde(default)]
    pub tools: Vec<TemaeTool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TemaePerson {
    pub person_id: Option<String>,
    pub name_free: Option<String>,
    pub role: String, // kyaku | hanto | mizuya | kengaku
    pub seat: Option<i64>,
    #[serde(default)]
    pub is_last: bool,
    pub role_note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TemaeTool {
    pub tool_id: Option<String>,
    pub name_free: Option<String>,
    pub external_ref: Option<String>,
}

/// Compact row for the home list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSummary {
    pub id: i64,
    pub date: Option<String>,
    pub date_prec: String,
    pub place_id: Option<String>,
    pub kind: String,
    pub online: bool,
    pub temae_count: i64,
}
