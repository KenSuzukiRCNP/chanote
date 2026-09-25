//! Profiles: several people using chanote on the same computer account, each with an
//! independent database + photo folder. No authentication — the data lives in the
//! OS user's own folder, so a password here would only be decorative.
//!
//! Layout under the app data root:
//!   profiles.json              this file
//!   chanote.sqlite, attachments/     the "default" profile (kept in place for upgrades)
//!   profiles/<id>/chanote.sqlite …   every other profile

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_ID: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub profiles: Vec<Profile>,
    /// show the picker at startup (only meaningful with 2+ profiles)
    #[serde(default)]
    pub ask_on_start: bool,
    /// profile opened last time
    #[serde(default = "default_id")]
    pub last: String,
}

fn default_id() -> String {
    DEFAULT_ID.into()
}

pub type Profiles = ProfilesFile;

impl ProfilesFile {
    pub fn load(root: &Path) -> std::io::Result<Self> {
        let p = root.join("profiles.json");
        if !p.exists() {
            return Ok(ProfilesFile {
                profiles: vec![Profile { id: DEFAULT_ID.into(), name: "Me".into() }],
                ask_on_start: false,
                last: DEFAULT_ID.into(),
            });
        }
        let text = std::fs::read_to_string(p)?;
        let mut f: ProfilesFile = serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if f.profiles.is_empty() {
            f.profiles.push(Profile { id: DEFAULT_ID.into(), name: "Me".into() });
        }
        if !f.profiles.iter().any(|p| p.id == f.last) {
            f.last = f.profiles[0].id.clone();
        }
        Ok(f)
    }

    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(root.join("profiles.json"), text)
    }

    pub fn add(&mut self, name: &str) -> Profile {
        let base: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let base = if base.is_empty() { "user".to_string() } else { base };
        let mut id = base.clone();
        let mut n = 2;
        while self.profiles.iter().any(|p| p.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let p = Profile { id, name: name.trim().to_string() };
        self.profiles.push(p.clone());
        p
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        match self.profiles.iter_mut().find(|p| p.id == id) {
            Some(p) => {
                p.name = name.trim().to_string();
                Ok(())
            }
            None => Err(format!("unknown profile {id}")),
        }
    }
}

pub fn data_dir_for(root: &Path, profile: &str) -> PathBuf {
    if profile == DEFAULT_ID {
        root.to_path_buf()
    } else {
        root.join("profiles").join(profile)
    }
}
