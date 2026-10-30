//! Folder sync. Each device keeps its own SQLite database; devices exchange *change logs*
//! through a folder that a cloud service (iCloud Drive, Dropbox, …) keeps in sync.
//!
//! Layout of the shared folder (`<parent>` is what the user picks):
//!
//! ```text
//! <parent>/chanote-sync/<diary-id>/
//!     diary.json                      {"app":"chanote","layout":1,"id":…,"name":…}
//!     log/<device-id>/00000001.json   one file per flush, written by that device only
//!     files/<attachment-uid>.<ext>    photo originals
//!     devices/<device-id>.json        last seen, platform (informational)
//! ```
//!
//! Every file is written by exactly one device and never modified afterwards, so a file
//! sync service never has to merge anything. A log file holds a batch of [`Event`]s.
//!
//! Records are identified across devices by `session.uid`, `attachment.uid` and the vocab
//! id. Each synced record has a revision `"<UTC time>|<counter>|<device>"` (`sync_rev`).
//! An event carries its new revision and the revision it was edited from (`base`):
//! - local revision == base (or none): fast-forward, apply.
//! - local revision == rev: already have it.
//! - otherwise both sides edited concurrently: the later revision wins on every device
//!   (deterministic), and the losing version is stored in `sync_conflict` so nothing
//!   disappears silently. Identical content is not reported as a conflict.

use crate::model::{Session, Vocab};
use crate::{Db, Error, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

pub const LAYOUT: u32 = 1;
pub const ROOT_DIR: &str = "chanote-sync";

// ---- events ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    SessionPut { uid: String, session: Session },
    SessionDel { uid: String },
    VocabPut { vocab: Vocab },
    VocabDel { id: String },
    AttachmentPut { uid: String, session_uid: String, ext: String, caption: Option<String>, created_at: String },
    AttachmentDel { uid: String },
}

impl Op {
    pub fn key(&self) -> (&'static str, String) {
        match self {
            Op::SessionPut { uid, .. } | Op::SessionDel { uid } => ("session", uid.clone()),
            Op::VocabPut { vocab } => ("vocab", vocab.id.clone()),
            Op::VocabDel { id } => ("vocab", id.clone()),
            Op::AttachmentPut { uid, .. } | Op::AttachmentDel { uid } => ("attachment", uid.clone()),
        }
    }
    fn is_delete(&self) -> bool {
        matches!(self, Op::SessionDel { .. } | Op::VocabDel { .. } | Op::AttachmentDel { .. })
    }
    /// Apply order inside one batch: names first (records refer to them), deletions of
    /// names last (so a record that still uses a name is applied before it goes).
    fn phase(&self) -> u8 {
        match self {
            Op::VocabPut { .. } => 0,
            Op::VocabDel { .. } => 2,
            _ => 1,
        }
    }
    fn label(&self) -> String {
        match self {
            Op::SessionPut { session, .. } => format!("{} {}", session.date.clone().unwrap_or_default(), session.form),
            Op::VocabPut { vocab } => format!("{} ({})", vocab.ja, vocab.kind),
            Op::AttachmentPut { caption, .. } => caption.clone().unwrap_or_else(|| "photo".into()),
            Op::SessionDel { .. } => "deleted record".into(),
            Op::VocabDel { id } => format!("deleted {id}"),
            Op::AttachmentDel { .. } => "deleted photo".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub rev: String,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(flatten)]
    pub op: Op,
}

#[derive(Debug, Serialize, Deserialize)]
struct LogFile {
    app: String,
    layout: u32,
    device: String,
    seq: u64,
    events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiaryInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    /// filled when listing: number of devices that have written to it
    #[serde(default)]
    pub devices: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncReport {
    pub sent: usize,
    pub received: usize,
    pub conflicts: usize,
    /// remote events waiting for something they refer to
    pub pending: usize,
    /// photos whose file has not arrived in the folder yet
    pub missing_files: usize,
    /// sessions adopted (matched by content) while joining
    pub adopted: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncStatus {
    pub enabled: bool,
    pub diary_id: Option<String>,
    pub diary_name: Option<String>,
    pub device: Option<String>,
    pub last_sync: Option<String>,
    pub outbox: i64,
    pub conflicts: i64,
    pub pending: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conflict {
    pub id: i64,
    pub kind: String,
    pub key: String,
    pub label: Option<String>,
    pub at: String,
    pub lost: Event,
}

/// Suppresses outbox writes while alive.
pub struct Quiet<'a> {
    db: &'a Db,
    prev: bool,
}
impl<'a> Quiet<'a> {
    pub fn new(db: &'a Db) -> Self {
        let prev = db.sync_quiet.swap(true, Ordering::SeqCst);
        Quiet { db, prev }
    }
}
impl Drop for Quiet<'_> {
    fn drop(&mut self) {
        self.db.sync_quiet.store(self.prev, Ordering::SeqCst);
    }
}

pub fn diary_dir(parent: &Path, id: &str) -> PathBuf {
    parent.join(ROOT_DIR).join(id)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("partial");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Diaries found under `<parent>/chanote-sync/`.
pub fn list_diaries(parent: &Path) -> Result<Vec<DiaryInfo>> {
    let root = parent.join(ROOT_DIR);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&root) else { return Ok(out) };
    for e in rd.flatten() {
        let p = e.path().join("diary.json");
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Ok(mut d) = serde_json::from_str::<DiaryInfo>(&text) {
                d.devices = std::fs::read_dir(e.path().join("log")).map(|r| r.flatten().count()).unwrap_or(0);
                out.push(d);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn read_diary(dir: &Path) -> Result<DiaryInfo> {
    let text = std::fs::read_to_string(dir.join("diary.json"))
        .map_err(|e| Error::Other(format!("no chanote diary at {}: {e}", dir.display())))?;
    Ok(serde_json::from_str(&text)?)
}

// ---- recording local changes --------------------------------------------------------

impl Db {
    fn sync_setting(&self, key: &str) -> Result<Option<String>> {
        self.get_setting(key)
    }

    /// Sync is on and we are not applying remote changes.
    pub fn sync_recording(&self) -> Result<bool> {
        if self.sync_quiet.load(Ordering::SeqCst) {
            return Ok(false);
        }
        Ok(self.sync_setting("sync_enabled")?.as_deref() == Some("1"))
    }

    fn sync_device(&self) -> Result<String> {
        self.sync_setting("sync_device")?.ok_or_else(|| Error::Other("sync device id missing".into()))
    }

    /// New revision string, strictly increasing on this device.
    fn sync_new_rev(&self) -> Result<String> {
        let device = self.sync_device()?;
        let n: i64 = self.sync_setting("sync_counter")?.and_then(|v| v.parse().ok()).unwrap_or(0) + 1;
        self.set_setting("sync_counter", &n.to_string())?;
        let now: String = self.conn().query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| r.get(0))?;
        Ok(format!("{now}|{n:010}|{device}"))
    }

    fn sync_rev_of(&self, kind: &str, key: &str) -> Result<Option<(String, bool)>> {
        Ok(self
            .conn()
            .query_row("SELECT rev, deleted FROM sync_rev WHERE kind = ?1 AND key = ?2", params![kind, key], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0))
            })
            .optional()?)
    }

    fn sync_set_rev(&self, kind: &str, key: &str, rev: &str, deleted: bool) -> Result<()> {
        self.conn().execute(
            "INSERT INTO sync_rev (kind, key, rev, deleted) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(kind, key) DO UPDATE SET rev = excluded.rev, deleted = excluded.deleted",
            params![kind, key, rev, deleted as i64],
        )?;
        Ok(())
    }

    /// Record a local change in the outbox (no-op unless sync is on).
    pub(crate) fn sync_note(&self, op: Op) -> Result<()> {
        if !self.sync_recording()? {
            return Ok(());
        }
        let (kind, key) = op.key();
        let base = self.sync_rev_of(kind, &key)?.map(|x| x.0);
        let rev = self.sync_new_rev()?;
        self.sync_set_rev(kind, &key, &rev, op.is_delete())?;
        let ev = Event { rev, base, op };
        self.conn().execute("INSERT INTO sync_outbox (event) VALUES (?1)", [serde_json::to_string(&ev)?])?;
        Ok(())
    }

    /// The session as it travels between devices: no local ids.
    pub(crate) fn sync_session_op(&self, id: i64) -> Result<Option<Op>> {
        let Some(mut s) = self.session_get(id)? else { return Ok(None) };
        let Some(uid) = s.uid.clone() else { return Ok(None) };
        s.id = None;
        for t in s.temae.iter_mut() {
            t.id = None;
        }
        Ok(Some(Op::SessionPut { uid, session: s }))
    }

    pub(crate) fn sync_note_session(&self, id: i64) -> Result<()> {
        if !self.sync_recording()? {
            return Ok(());
        }
        if let Some(op) = self.sync_session_op(id)? {
            self.sync_note(op)?;
        }
        Ok(())
    }

    /// Which vocab rows travel: the user's own rows (except "self", which every device has),
    /// pack rows the user edited (pinned), and anything that has synced before.
    fn sync_vocab_eligible(&self, v: &Vocab) -> Result<bool> {
        let meta = v.meta.as_ref();
        if meta.and_then(|m| m.get("is_self")).and_then(|x| x.as_bool()) == Some(true) {
            return Ok(false);
        }
        if v.origin == "user" || meta.and_then(|m| m.get("pinned")).and_then(|x| x.as_bool()) == Some(true) {
            return Ok(true);
        }
        Ok(self.sync_rev_of("vocab", &v.id)?.is_some())
    }

    pub(crate) fn sync_note_vocab(&self, id: &str) -> Result<()> {
        if !self.sync_recording()? {
            return Ok(());
        }
        if let Some(v) = self.vocab_get(id)? {
            if self.sync_vocab_eligible(&v)? {
                self.sync_note(Op::VocabPut { vocab: v })?;
            }
        }
        Ok(())
    }

    fn sync_attachment_op(&self, id: i64) -> Result<Option<Op>> {
        let row: Option<(Option<String>, Option<i64>, String, Option<String>, String)> = self
            .conn()
            .query_row(
                "SELECT uid, session_id, path, caption, created_at FROM attachment WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()?;
        let Some((Some(uid), Some(sid), path, caption, created_at)) = row else { return Ok(None) };
        let Some(session_uid) = self.session_uid(sid)? else { return Ok(None) };
        let ext = Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_ascii_lowercase();
        Ok(Some(Op::AttachmentPut { uid, session_uid, ext, caption, created_at }))
    }

    pub(crate) fn sync_note_attachment(&self, id: i64) -> Result<()> {
        if !self.sync_recording()? {
            return Ok(());
        }
        if let Some(op) = self.sync_attachment_op(id)? {
            self.sync_note(op)?;
        }
        Ok(())
    }

    /// Current local state of a record, as the op that would recreate it.
    fn sync_current(&self, kind: &str, key: &str) -> Result<Option<Op>> {
        match kind {
            "session" => match self.session_id_by_uid(key)? {
                Some(id) => self.sync_session_op(id),
                None => Ok(None),
            },
            "vocab" => Ok(self.vocab_get(key)?.map(|v| Op::VocabPut { vocab: v })),
            "attachment" => {
                let id: Option<i64> = self.conn().query_row("SELECT id FROM attachment WHERE uid = ?1", [key], |r| r.get(0)).optional()?;
                match id {
                    Some(id) => self.sync_attachment_op(id),
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }
}

// ---- applying remote changes ------------------------------------------------------

enum Applied {
    Applied,
    Skipped,
    Conflict,
}

impl Db {
    fn sync_record_conflict(&self, kind: &str, key: &str, kept: &str, lost: &Event) -> Result<()> {
        self.conn().execute(
            "INSERT INTO sync_conflict (kind, key, label, kept_rev, lost_rev, lost) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![kind, key, lost.op.label(), kept, lost.rev, serde_json::to_string(lost)?],
        )?;
        Ok(())
    }

    fn sync_apply(&self, ev: &Event, files: &Path, data_dir: &Path) -> Result<Applied> {
        let (kind, key) = ev.op.key();
        let local = self.sync_rev_of(kind, &key)?;
        let mut outcome = Applied::Applied;
        if let Some((local_rev, _)) = &local {
            if *local_rev == ev.rev {
                return Ok(Applied::Skipped);
            }
            let fast_forward = ev.base.as_deref() == Some(local_rev.as_str());
            if !fast_forward {
                // concurrent edits: the later revision wins everywhere
                let current = self.sync_current(kind, &key)?;
                let same = match (&current, &ev.op) {
                    (Some(c), o) => c == o,
                    (None, o) => o.is_delete(),
                };
                if local_rev.as_str() > ev.rev.as_str() {
                    if !same {
                        self.sync_record_conflict(kind, &key, local_rev, ev)?;
                        return Ok(Applied::Conflict);
                    }
                    return Ok(Applied::Skipped);
                }
                if !same {
                    let lost = Event {
                        rev: local_rev.clone(),
                        base: None,
                        op: current.unwrap_or_else(|| deleted_op(kind, &key)),
                    };
                    self.sync_record_conflict(kind, &key, &ev.rev, &lost)?;
                    outcome = Applied::Conflict;
                }
            }
        }
        if local.is_none() && !ev.op.is_delete() {
            // a record this device has but never synced (same vocab id, or a session adopted
            // while joining): keep a copy of ours if it differs
            if let Some(current) = self.sync_current(kind, &key)? {
                if current != ev.op {
                    let lost = Event { rev: "local".into(), base: None, op: current };
                    self.sync_record_conflict(kind, &key, &ev.rev, &lost)?;
                    outcome = Applied::Conflict;
                }
            }
        }
        let _q = Quiet::new(self);
        self.sync_perform(&ev.op, files, data_dir)?;
        self.sync_set_rev(kind, &key, &ev.rev, ev.op.is_delete())?;
        Ok(outcome)
    }

    fn sync_perform(&self, op: &Op, files: &Path, data_dir: &Path) -> Result<()> {
        match op {
            Op::SessionPut { uid, session } => {
                let mut s = session.clone();
                s.uid = Some(uid.clone());
                s.id = self.session_id_by_uid(uid)?;
                self.session_save(&s)?;
            }
            Op::SessionDel { uid } => {
                if let Some(id) = self.session_id_by_uid(uid)? {
                    self.session_delete(id)?;
                }
            }
            Op::VocabPut { vocab } => self.vocab_upsert(vocab)?,
            Op::VocabDel { id } => {
                if self.vocab_get(id)?.is_some() && self.vocab_delete(id).is_err() {
                    // still used here (e.g. by a record not yet synced): hide it instead
                    self.vocab_archive(id, true)?;
                }
            }
            Op::AttachmentPut { uid, session_uid, ext, caption, created_at } => {
                let existing: Option<i64> =
                    self.conn().query_row("SELECT id FROM attachment WHERE uid = ?1", [uid], |r| r.get(0)).optional()?;
                match existing {
                    Some(id) => {
                        self.conn().execute("UPDATE attachment SET caption = ?2 WHERE id = ?1", params![id, caption])?;
                    }
                    None => {
                        let sid = self
                            .session_id_by_uid(session_uid)?
                            .ok_or_else(|| Error::Other(format!("photo for unknown record {session_uid}")))?;
                        self.conn().execute(
                            "INSERT INTO attachment (session_id, path, caption, created_at, uid) VALUES (?1, '', ?2, ?3, ?4)",
                            params![sid, caption, created_at, uid],
                        )?;
                        let id = self.conn().last_insert_rowid();
                        self.sync_fetch_file(id, uid, ext, files, data_dir)?;
                    }
                }
            }
            Op::AttachmentDel { uid } => {
                let id: Option<i64> =
                    self.conn().query_row("SELECT id FROM attachment WHERE uid = ?1", [uid], |r| r.get(0)).optional()?;
                if let Some(id) = id {
                    self.attachment_delete(data_dir, id)?;
                }
            }
        }
        Ok(())
    }

    /// Copy a photo from the sync folder into the data dir (if it has arrived).
    fn sync_fetch_file(&self, id: i64, uid: &str, ext: &str, files: &Path, data_dir: &Path) -> Result<bool> {
        let src = files.join(format!("{uid}.{ext}"));
        if !src.exists() {
            return Ok(false);
        }
        let year: String = self.conn().query_row(
            "SELECT COALESCE(substr(s.date,1,4), 'undated') FROM attachment a JOIN session s ON s.id = a.session_id WHERE a.id = ?1",
            [id],
            |r| r.get(0),
        )?;
        let rel = format!("attachments/{year}/att-{id}.{ext}");
        let dst = data_dir.join(&rel);
        if let Some(dir) = dst.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&src, &dst)?;
        let _ = crate::attachment::make_thumbnail(&dst, &crate::attachment::thumb_path(data_dir, id));
        self.conn().execute("UPDATE attachment SET path = ?2 WHERE id = ?1", params![id, rel])?;
        Ok(true)
    }
}

fn deleted_op(kind: &str, key: &str) -> Op {
    match kind {
        "session" => Op::SessionDel { uid: key.into() },
        "vocab" => Op::VocabDel { id: key.into() },
        _ => Op::AttachmentDel { uid: key.into() },
    }
}

// ---- the sync folder ---------------------------------------------------------------

impl Db {
    pub fn sync_status(&self) -> Result<SyncStatus> {
        let count = |sql: &str| -> Result<i64> { Ok(self.conn().query_row(sql, [], |r| r.get(0))?) };
        Ok(SyncStatus {
            enabled: self.sync_setting("sync_enabled")?.as_deref() == Some("1"),
            diary_id: self.sync_setting("sync_diary")?,
            diary_name: self.sync_setting("sync_diary_name")?,
            device: self.sync_setting("sync_device")?,
            last_sync: self.sync_setting("sync_last")?,
            outbox: count("SELECT COUNT(*) FROM sync_outbox")?,
            conflicts: count("SELECT COUNT(*) FROM sync_conflict WHERE resolved = 0")?,
            pending: count("SELECT COUNT(*) FROM sync_pending")?,
        })
    }

    /// Create a new diary in `<parent>/chanote-sync/` and start syncing this database to it.
    pub fn sync_create(&self, parent: &Path, name: &str, data_dir: &Path, device_label: &str) -> Result<SyncReport> {
        let id: String = self.conn().query_row("SELECT lower(hex(randomblob(8)))", [], |r| r.get(0))?;
        let now: String = self.conn().query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |r| r.get(0))?;
        let info = DiaryInfo { id: id.clone(), name: name.to_string(), created_at: now, devices: 0 };
        let dir = diary_dir(parent, &id);
        let mut doc = serde_json::to_value(&info)?;
        doc["app"] = "chanote".into();
        doc["layout"] = LAYOUT.into();
        write_atomic(&dir.join("diary.json"), serde_json::to_string_pretty(&doc)?.as_bytes())?;
        self.sync_join(parent, &id, data_dir, device_label)
    }

    /// Join an existing diary (or the one just created). Remote records are applied first;
    /// local records that match one by date, place and procedures adopt its id instead of
    /// being sent twice; then everything else local is sent.
    pub fn sync_join(&self, parent: &Path, id: &str, data_dir: &Path, device_label: &str) -> Result<SyncReport> {
        let dir = diary_dir(parent, id);
        let info = read_diary(&dir)?;
        // fresh identity for this database in this diary
        self.sync_reset()?;
        let device: String = self.conn().query_row("SELECT lower(hex(randomblob(6)))", [], |r| r.get(0))?;
        self.set_setting("sync_device", &device)?;
        self.set_setting("sync_device_label", device_label)?;
        self.set_setting("sync_diary", &info.id)?;
        self.set_setting("sync_diary_name", &info.name)?;
        self.set_setting("sync_counter", "0")?;
        self.set_setting("sync_file_seq", "0")?;

        // 1. what the others already have (not recorded: sync is still off)
        let mut rep = SyncReport::default();
        let (events, seen) = self.sync_collect(&dir, &device)?;
        rep.adopted = self.sync_adopt(&events)?;
        self.sync_apply_all(events, &dir, data_dir, &mut rep)?;
        self.sync_store_seen(&seen)?;

        // 2. switch on, and send everything local the folder does not have yet
        self.set_setting("sync_enabled", "1")?;
        self.sync_snapshot()?;
        let more = self.sync_run(parent, data_dir)?;
        rep.sent += more.sent;
        rep.received += more.received;
        rep.conflicts += more.conflicts;
        rep.pending = more.pending;
        rep.missing_files = more.missing_files;
        Ok(rep)
    }

    /// Stop syncing and forget all sync bookkeeping (records stay).
    pub fn sync_reset(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            "DELETE FROM sync_rev; DELETE FROM sync_outbox; DELETE FROM sync_seen; DELETE FROM sync_pending;
             DELETE FROM setting WHERE key IN ('sync_enabled','sync_device','sync_diary','sync_diary_name',
                'sync_counter','sync_file_seq','sync_last','sync_device_label');",
        )?;
        Ok(())
    }

    /// Local sessions with the same fingerprint as an incoming one take over its uid.
    fn sync_adopt(&self, events: &[Event]) -> Result<usize> {
        let mut local: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        let ids: Vec<i64> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT id FROM session")?;
            let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        for id in ids {
            if let Some(s) = self.session_get(id)? {
                local.insert(crate::import::fingerprint(&s), id);
            }
        }
        let mut n = 0;
        for ev in events {
            if let Op::SessionPut { uid, session } = &ev.op {
                if self.session_id_by_uid(uid)?.is_some() {
                    continue;
                }
                if let Some(id) = local.remove(&crate::import::fingerprint(session)) {
                    self.conn().execute("UPDATE session SET uid = ?2 WHERE id = ?1", params![id, uid])?;
                    n += 1;
                }
            }
        }
        Ok(n)
    }

    /// Put every local record that has no revision yet into the outbox.
    fn sync_snapshot(&self) -> Result<()> {
        let vocab_ids: Vec<String> = {
            let conn = self.conn();
            let mut st = conn.prepare(
                "SELECT id FROM vocab v WHERE NOT EXISTS (SELECT 1 FROM sync_rev r WHERE r.kind = 'vocab' AND r.key = v.id)
                 ORDER BY CASE WHEN parent_id IS NULL THEN 0 ELSE 1 END, kind, sort, id",
            )?;
            let v: Vec<String> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        for id in vocab_ids {
            self.sync_note_vocab(&id)?;
        }
        let session_ids: Vec<i64> = {
            let conn = self.conn();
            let mut st = conn.prepare(
                "SELECT id FROM session s WHERE NOT EXISTS (SELECT 1 FROM sync_rev r WHERE r.kind = 'session' AND r.key = s.uid)
                 ORDER BY date, id",
            )?;
            let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        for id in session_ids {
            self.sync_note_session(id)?;
        }
        let att_ids: Vec<i64> = {
            let conn = self.conn();
            let mut st = conn.prepare(
                "SELECT id FROM attachment a WHERE path <> '' AND NOT EXISTS (SELECT 1 FROM sync_rev r WHERE r.kind = 'attachment' AND r.key = a.uid)",
            )?;
            let v: Vec<i64> = st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        for id in att_ids {
            self.sync_note_attachment(id)?;
        }
        Ok(())
    }

    /// One round: send the outbox, apply what the other devices wrote, retry anything that
    /// was waiting, fetch photo files that have arrived.
    pub fn sync_run(&self, parent: &Path, data_dir: &Path) -> Result<SyncReport> {
        let id = self.sync_setting("sync_diary")?.ok_or_else(|| Error::Other("sync is not set up".into()))?;
        if self.sync_setting("sync_enabled")?.as_deref() != Some("1") {
            return Err(Error::Other("sync is off".into()));
        }
        let dir = diary_dir(parent, &id);
        read_diary(&dir)?; // folder still there?
        let device = self.sync_device()?;
        let mut rep = SyncReport::default();

        // 1. send
        rep.sent = self.sync_flush(&dir, &device, data_dir)?;

        // 2. receive
        let (events, seen) = self.sync_collect(&dir, &device)?;
        self.sync_apply_all(events, &dir, data_dir, &mut rep)?;
        self.sync_store_seen(&seen)?;

        // 3. photos that arrived late
        rep.missing_files = self.sync_fill_missing(&dir, data_dir)?;

        // 4. who am I (informational)
        let now: String = self.conn().query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |r| r.get(0))?;
        let label = self.sync_setting("sync_device_label")?.unwrap_or_default();
        let _ = write_atomic(
            &dir.join("devices").join(format!("{device}.json")),
            serde_json::to_string_pretty(&serde_json::json!({"device": device, "label": label, "last_sync": now}))?.as_bytes(),
        );
        self.set_setting("sync_last", &now)?;
        rep.pending = self.conn().query_row("SELECT COUNT(*) FROM sync_pending", [], |r| r.get::<_, i64>(0))? as usize;
        Ok(rep)
    }

    fn sync_flush(&self, dir: &Path, device: &str, data_dir: &Path) -> Result<usize> {
        let rows: Vec<(i64, String)> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT seq, event FROM sync_outbox ORDER BY seq")?;
            let v: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        if rows.is_empty() {
            return Ok(0);
        }
        let events: Vec<Event> = rows.iter().map(|(_, e)| serde_json::from_str(e)).collect::<std::result::Result<_, _>>()?;
        // photo originals first, so a receiver that sees the event usually finds the file
        for ev in &events {
            if let Op::AttachmentPut { uid, ext, .. } = &ev.op {
                let dst = dir.join("files").join(format!("{uid}.{ext}"));
                if !dst.exists() {
                    let rel: Option<String> = self
                        .conn()
                        .query_row("SELECT path FROM attachment WHERE uid = ?1", [uid], |r| r.get(0))
                        .optional()?;
                    if let Some(rel) = rel.filter(|r| !r.is_empty()) {
                        let src = data_dir.join(rel);
                        if src.exists() {
                            std::fs::create_dir_all(dst.parent().unwrap())?;
                            let tmp = dst.with_extension("partial");
                            std::fs::copy(&src, &tmp)?;
                            std::fs::rename(&tmp, &dst)?;
                        }
                    }
                }
            }
        }
        let seq: u64 = self.sync_setting("sync_file_seq")?.and_then(|v| v.parse().ok()).unwrap_or(0) + 1;
        let file = LogFile { app: "chanote".into(), layout: LAYOUT, device: device.into(), seq, events };
        let path = dir.join("log").join(device).join(format!("{seq:08}.json"));
        write_atomic(&path, serde_json::to_string(&file)?.as_bytes())?;
        self.set_setting("sync_file_seq", &seq.to_string())?;
        let last = rows.last().unwrap().0;
        self.conn().execute("DELETE FROM sync_outbox WHERE seq <= ?1", [last])?;
        Ok(file.events.len())
    }

    /// New events from the other devices, in file order. Files are taken contiguously from
    /// the last one applied: if the cloud delivered file 5 before file 4, we wait for 4.
    fn sync_collect(&self, dir: &Path, me: &str) -> Result<(Vec<Event>, Vec<(String, u64)>)> {
        let mut events = Vec::new();
        let mut seen = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir.join("log")) else { return Ok((events, seen)) };
        for e in rd.flatten() {
            let device = e.file_name().to_string_lossy().to_string();
            if device == me || !e.path().is_dir() {
                continue;
            }
            let mut last: u64 = self
                .conn()
                .query_row("SELECT file_seq FROM sync_seen WHERE device = ?1", [&device], |r| r.get::<_, i64>(0))
                .optional()?
                .unwrap_or(0) as u64;
            let start = last;
            loop {
                let p = e.path().join(format!("{:08}.json", last + 1));
                let Ok(text) = std::fs::read_to_string(&p) else { break };
                let Ok(file) = serde_json::from_str::<LogFile>(&text) else { break }; // partially synced: retry later
                events.extend(file.events);
                last += 1;
            }
            if last > start {
                seen.push((device, last));
            }
        }
        Ok((events, seen))
    }

    fn sync_store_seen(&self, seen: &[(String, u64)]) -> Result<()> {
        for (d, n) in seen {
            self.conn().execute(
                "INSERT INTO sync_seen (device, file_seq) VALUES (?1, ?2)
                 ON CONFLICT(device) DO UPDATE SET file_seq = excluded.file_seq",
                params![d, *n as i64],
            )?;
        }
        Ok(())
    }

    fn sync_apply_all(&self, mut events: Vec<Event>, dir: &Path, data_dir: &Path, rep: &mut SyncReport) -> Result<()> {
        // earlier waiting events go first
        let pending: Vec<(i64, String)> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT id, event FROM sync_pending ORDER BY id")?;
            let v: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        self.conn().execute("DELETE FROM sync_pending", [])?;
        let mut all: Vec<Event> = pending.into_iter().filter_map(|(_, e)| serde_json::from_str(&e).ok()).collect();
        all.append(&mut events);
        all.sort_by(|a, b| a.op.phase().cmp(&b.op.phase()).then_with(|| a.rev.cmp(&b.rev)));
        let files = dir.join("files");
        for ev in all {
            match self.sync_apply(&ev, &files, data_dir) {
                Ok(Applied::Applied) => rep.received += 1,
                Ok(Applied::Conflict) => {
                    rep.received += 1;
                    rep.conflicts += 1;
                }
                Ok(Applied::Skipped) => {}
                Err(e) => {
                    self.conn().execute(
                        "INSERT INTO sync_pending (event, error) VALUES (?1, ?2)",
                        params![serde_json::to_string(&ev)?, e.to_string()],
                    )?;
                }
            }
        }
        Ok(())
    }

    fn sync_fill_missing(&self, dir: &Path, data_dir: &Path) -> Result<usize> {
        let rows: Vec<(i64, String)> = {
            let conn = self.conn();
            let mut st = conn.prepare("SELECT id, uid FROM attachment WHERE path = '' AND uid IS NOT NULL")?;
            let v: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
            v
        };
        let files = dir.join("files");
        let mut missing = 0;
        for (id, uid) in rows {
            // extension from whatever file with that uid is there
            let found = std::fs::read_dir(&files).ok().and_then(|rd| {
                rd.flatten().find_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_prefix(&format!("{uid}.")).filter(|ext| !ext.contains('.') && *ext != "partial").map(String::from)
                })
            });
            match found {
                Some(ext) if self.sync_fetch_file(id, &uid, &ext, &files, data_dir)? => {}
                _ => missing += 1,
            }
        }
        Ok(missing)
    }

    pub fn sync_conflicts(&self) -> Result<Vec<Conflict>> {
        let conn = self.conn();
        let mut st = conn.prepare("SELECT id, kind, key, label, at, lost FROM sync_conflict WHERE resolved = 0 ORDER BY id DESC")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (id, kind, key, label, at, lost) = r?;
            if let Ok(lost) = serde_json::from_str(&lost) {
                out.push(Conflict { id, kind, key, label, at, lost });
            }
        }
        Ok(out)
    }

    /// `restore = true`: bring the losing version back as a new edit on this device (it is
    /// then sent to the others). Either way the conflict is marked resolved.
    pub fn sync_conflict_resolve(&self, id: i64, restore: bool, data_dir: &Path) -> Result<()> {
        let lost: String = self.conn().query_row("SELECT lost FROM sync_conflict WHERE id = ?1", [id], |r| r.get(0))?;
        if restore {
            let ev: Event = serde_json::from_str(&lost)?;
            match &ev.op {
                Op::SessionPut { uid, session } => {
                    let mut s = session.clone();
                    s.uid = Some(uid.clone());
                    s.id = self.session_id_by_uid(uid)?;
                    self.session_save(&s)?;
                }
                Op::VocabPut { vocab } => self.vocab_upsert(vocab)?,
                Op::SessionDel { uid } => {
                    if let Some(sid) = self.session_id_by_uid(uid)? {
                        self.session_delete(sid)?;
                    }
                }
                Op::VocabDel { id } => {
                    let _ = self.vocab_delete(id);
                }
                Op::AttachmentPut { uid, caption, .. } => {
                    let aid: Option<i64> = self.conn().query_row("SELECT id FROM attachment WHERE uid = ?1", [uid], |r| r.get(0)).optional()?;
                    if let Some(aid) = aid {
                        self.attachment_set_caption(aid, caption.as_deref())?;
                    }
                }
                Op::AttachmentDel { uid } => {
                    let aid: Option<i64> = self.conn().query_row("SELECT id FROM attachment WHERE uid = ?1", [uid], |r| r.get(0)).optional()?;
                    if let Some(aid) = aid {
                        self.attachment_delete(data_dir, aid)?;
                    }
                }
            }
        }
        self.conn().execute("UPDATE sync_conflict SET resolved = 1 WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionPerson, Temae};
    use crate::pack::{Pack, URASENKE_YAML};

    fn device() -> Db {
        let db = Db::open_in_memory().unwrap();
        db.pack_install(&Pack::from_yaml(URASENKE_YAML).unwrap()).unwrap();
        db.self_person_id().unwrap();
        db
    }
    fn person(db: &Db, id: &str, ja: &str) {
        db.vocab_upsert(&Vocab { id: id.into(), kind: "person".into(), ja: ja.into(), romaji: None, en: None, parent_id: None, sort: None, origin: "user".into(), archived: false, meta: None }).unwrap();
    }
    fn lesson(date: &str, temae: &str, guest: Option<&str>) -> Session {
        Session {
            date: Some(date.into()),
            persons: guest.map(|g| vec![SessionPerson { person_id: Some(g.into()), role: "guest".into(), seat: Some(1), ..Default::default() }]).unwrap_or_default(),
            temae: vec![Temae { temae_id: Some(temae.into()), host_id: Some("person.self".into()), my_role: Some("teishu".into()), ..Default::default() }],
            ..Default::default()
        }
    }
    fn sessions(db: &Db) -> Vec<Session> {
        db.session_list(1000, 0).unwrap().into_iter().map(|r| db.session_get(r.id).unwrap().unwrap()).collect()
    }

    #[test]
    fn two_devices_sync_through_a_folder() {
        let folder = tempfile::tempdir().unwrap();
        let (da, db_dir) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let mac = device();
        let phone = device();

        // the Mac has history, then creates the diary
        person(&mac, "person.sato-x1", "佐藤さん");
        mac.session_save(&lesson("2026-01-10", "temae.urasenke.karamono", Some("person.sato-x1"))).unwrap();
        mac.session_save(&lesson("2026-01-17", "temae.urasenke.hiradate", None)).unwrap();
        let rep = mac.sync_create(folder.path(), "Ken", da.path(), "Mac").unwrap();
        assert_eq!(rep.sent, 3, "{rep:?}"); // one person + two sessions
        let diaries = list_diaries(folder.path()).unwrap();
        assert_eq!(diaries.len(), 1);
        assert_eq!(diaries[0].name, "Ken");

        // the phone already holds one of those lessons (imported from a JSON export earlier)
        person(&phone, "person.sato-x1", "佐藤さん");
        phone.session_save(&lesson("2026-01-10", "temae.urasenke.karamono", Some("person.sato-x1"))).unwrap();
        let rep = phone.sync_join(folder.path(), &diaries[0].id, db_dir.path(), "iPhone").unwrap();
        assert_eq!(rep.adopted, 1, "{rep:?}");
        assert_eq!(sessions(&phone).len(), 2, "no duplicate of the adopted lesson");
        assert_eq!(rep.conflicts, 0);

        // edits travel both ways
        let mut s = sessions(&phone).into_iter().find(|s| s.date.as_deref() == Some("2026-01-17")).unwrap();
        s.note = Some("from the phone".into());
        phone.session_save(&s).unwrap();
        phone.session_save(&lesson("2026-02-01", "temae.urasenke.kagetsu", None)).unwrap();
        phone.sync_run(folder.path(), db_dir.path()).unwrap();
        let rep = mac.sync_run(folder.path(), da.path()).unwrap();
        assert_eq!(rep.received, 2, "{rep:?}");
        let on_mac = sessions(&mac);
        assert_eq!(on_mac.len(), 3);
        assert!(on_mac.iter().any(|s| s.note.as_deref() == Some("from the phone")));

        // deletion travels and does not come back
        let feb = on_mac.iter().find(|s| s.date.as_deref() == Some("2026-02-01")).unwrap();
        mac.session_delete(feb.id.unwrap()).unwrap();
        mac.sync_run(folder.path(), da.path()).unwrap();
        phone.sync_run(folder.path(), db_dir.path()).unwrap();
        assert_eq!(sessions(&phone).len(), 2);
        phone.sync_run(folder.path(), db_dir.path()).unwrap();
        mac.sync_run(folder.path(), da.path()).unwrap();
        assert_eq!(sessions(&mac).len(), 2);

        // nothing left to do: a second round is quiet
        let r1 = mac.sync_run(folder.path(), da.path()).unwrap();
        let r2 = phone.sync_run(folder.path(), db_dir.path()).unwrap();
        assert_eq!((r1.sent, r1.received, r2.sent, r2.received), (0, 0, 0, 0));
    }

    #[test]
    fn concurrent_edits_keep_the_later_and_record_the_other() {
        let folder = tempfile::tempdir().unwrap();
        let (da, dbb) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let a = device();
        let b = device();
        a.session_save(&lesson("2026-03-01", "temae.urasenke.karamono", None)).unwrap();
        a.sync_create(folder.path(), "Ken", da.path(), "A").unwrap();
        let id = list_diaries(folder.path()).unwrap()[0].id.clone();
        b.sync_join(folder.path(), &id, dbb.path(), "B").unwrap();

        // both edit the same record before syncing
        let mut sa = sessions(&a)[0].clone();
        sa.note = Some("A's note".into());
        a.session_save(&sa).unwrap();
        let mut sb = sessions(&b)[0].clone();
        sb.note = Some("B's note".into());
        std::thread::sleep(std::time::Duration::from_millis(5));
        b.session_save(&sb).unwrap();

        a.sync_run(folder.path(), da.path()).unwrap();
        let rb = b.sync_run(folder.path(), dbb.path()).unwrap();
        let ra = a.sync_run(folder.path(), da.path()).unwrap();
        assert_eq!(rb.conflicts + ra.conflicts, 2, "each side records the version it did not keep");
        // both converge on B's later edit
        assert_eq!(sessions(&a)[0].note.as_deref(), Some("B's note"));
        assert_eq!(sessions(&b)[0].note.as_deref(), Some("B's note"));
        // A can bring its version back; that becomes a normal edit that reaches B
        let c = a.sync_conflicts().unwrap();
        assert_eq!(c.len(), 1);
        a.sync_conflict_resolve(c[0].id, true, da.path()).unwrap();
        assert_eq!(sessions(&a)[0].note.as_deref(), Some("A's note"));
        a.sync_run(folder.path(), da.path()).unwrap();
        b.sync_run(folder.path(), dbb.path()).unwrap();
        assert_eq!(sessions(&b)[0].note.as_deref(), Some("A's note"));
        assert!(a.sync_conflicts().unwrap().is_empty());
    }

    #[test]
    fn photos_travel_with_their_file_and_late_files_are_fetched() {
        let folder = tempfile::tempdir().unwrap();
        let (da, dbb) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let a = device();
        let b = device();
        let sid = a.session_save(&lesson("2026-04-01", "temae.urasenke.hiradate", None)).unwrap();
        let src = da.path().join("pic.png");
        image::RgbImage::new(8, 6).save(&src).unwrap();
        a.sync_create(folder.path(), "Ken", da.path(), "A").unwrap();
        let att = a.attachment_add(da.path(), sid, &src).unwrap();
        a.attachment_set_caption(att.id, Some("床")).unwrap();
        a.sync_run(folder.path(), da.path()).unwrap();
        let id = list_diaries(folder.path()).unwrap()[0].id.clone();

        // simulate the cloud delivering the log before the photo
        let files = diary_dir(folder.path(), &id).join("files");
        let f = std::fs::read_dir(&files).unwrap().next().unwrap().unwrap().path();
        let hidden = folder.path().join("held-back");
        std::fs::rename(&f, &hidden).unwrap();
        let rep = b.sync_join(folder.path(), &id, dbb.path(), "B").unwrap();
        assert_eq!(rep.missing_files, 1, "{rep:?}");
        let s = sessions(&b)[0].clone();
        let list = b.attachment_list(s.id.unwrap()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].caption.as_deref(), Some("床"));
        assert_eq!(list[0].path, "");

        std::fs::rename(&hidden, &f).unwrap();
        let rep = b.sync_run(folder.path(), dbb.path()).unwrap();
        assert_eq!(rep.missing_files, 0);
        let list = b.attachment_list(s.id.unwrap()).unwrap();
        assert!(dbb.path().join(&list[0].path).exists());
        assert!(b.attachment_thumb_data_url(dbb.path(), list[0].id).unwrap().is_some());
    }

    #[test]
    fn out_of_order_files_wait_and_names_arrive_before_records() {
        let folder = tempfile::tempdir().unwrap();
        let (da, dbb) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let a = device();
        let b = device();
        a.sync_create(folder.path(), "Ken", da.path(), "A").unwrap();
        let id = list_diaries(folder.path()).unwrap()[0].id.clone();
        b.sync_join(folder.path(), &id, dbb.path(), "B").unwrap();

        // file 1 (a new person) and file 2 (a record using them) — deliver only file 2 first
        person(&a, "person.new-1", "新しい人");
        a.sync_run(folder.path(), da.path()).unwrap();
        a.session_save(&lesson("2026-05-01", "temae.urasenke.hiradate", Some("person.new-1"))).unwrap();
        a.sync_run(folder.path(), da.path()).unwrap();
        let dev = a.sync_status().unwrap().device.unwrap();
        let logdir = diary_dir(folder.path(), &id).join("log").join(&dev);
        let mut names: Vec<_> = std::fs::read_dir(&logdir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.sort();
        let first_new = logdir.join(&names[names.len() - 2]);
        let held = folder.path().join("held.json");
        std::fs::rename(&first_new, &held).unwrap();
        let rep = b.sync_run(folder.path(), dbb.path()).unwrap();
        assert_eq!(rep.received, 0, "file 2 waits for file 1");
        std::fs::rename(&held, &first_new).unwrap();
        let rep = b.sync_run(folder.path(), dbb.path()).unwrap();
        assert_eq!(rep.received, 2);
        assert_eq!(rep.pending, 0);
        assert!(b.vocab_get("person.new-1").unwrap().is_some());
        assert_eq!(sessions(&b).len(), 1);
        // pack rows never travel
        let ev_text = std::fs::read_to_string(logdir.join(&names[0])).unwrap();
        assert!(!ev_text.contains("temae.urasenke.karamono\",\"kind\":\"temae\""));
    }
}
