//! Photos attached to a session. Files are copied into `<data_dir>/attachments/YYYY/`
//! and a JPEG thumbnail is written to `<data_dir>/attachments/thumbs/`. The database
//! stores paths relative to `<data_dir>`.

use crate::{Db, Error, Result};
use base64::Engine;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const THUMB_MAX: u32 = 480;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attachment {
    pub id: i64,
    pub session_id: Option<i64>,
    pub temae_id: Option<i64>,
    /// relative to the data dir
    pub path: String,
    pub caption: Option<String>,
    pub created_at: String,
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        session_id: r.get(1)?,
        temae_id: r.get(2)?,
        path: r.get(3)?,
        caption: r.get(4)?,
        created_at: r.get(5)?,
    })
}

const COLS: &str = "id, session_id, temae_id, path, caption, created_at";

impl Db {
    /// Copy `src` into the data dir, create a thumbnail, and record it on `session_id`.
    /// HEIC/HEIF sources are converted to JPEG first (macOS only, via `sips`).
    pub fn attachment_add(&self, data_dir: &Path, session_id: i64, src: &Path) -> Result<Attachment> {
        let mut ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "bin".into());
        // HEIC → JPEG
        let converted;
        let src: &Path = if ext == "heic" || ext == "heif" {
            converted = convert_heic_to_jpeg(src, data_dir)?;
            ext = "jpg".into();
            &converted
        } else {
            src
        };
        let year: String = {
            let conn = self.conn();
            conn.query_row("SELECT COALESCE(substr(date,1,4), 'undated') FROM session WHERE id = ?1", [session_id], |r| r.get(0))
                .map_err(|_| Error::Other(format!("session {session_id} not found")))?
        };

        // reserve the row to get an id for the file name
        let id: i64 = {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO attachment (session_id, path) VALUES (?1, '')",
                [session_id],
            )?;
            conn.last_insert_rowid()
        };
        let rel = format!("attachments/{year}/att-{id}.{ext}");
        let dst = data_dir.join(&rel);
        if let Some(dir) = dst.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(src, &dst)?;
        if src.starts_with(data_dir.join(".tmp")) {
            let _ = std::fs::remove_file(src);
        }

        // thumbnail (best effort: unsupported formats just have no thumb)
        let thumb_dir = data_dir.join("attachments/thumbs");
        std::fs::create_dir_all(&thumb_dir)?;
        if let Ok(img) = image::open(&dst) {
            let t = img.thumbnail(THUMB_MAX, THUMB_MAX);
            let _ = t.to_rgb8().save(thumb_dir.join(format!("att-{id}.jpg")));
        }

        self.conn().execute("UPDATE attachment SET path = ?2 WHERE id = ?1", params![id, rel])?;
        self.attachment_get(id)?.ok_or_else(|| Error::Other("attachment vanished".into()))
    }

    pub fn attachment_get(&self, id: i64) -> Result<Option<Attachment>> {
        let conn = self.conn();
        let sql = format!("SELECT {COLS} FROM attachment WHERE id = ?1");
        Ok(conn.query_row(&sql, [id], row).ok())
    }

    pub fn attachment_list(&self, session_id: i64) -> Result<Vec<Attachment>> {
        let conn = self.conn();
        let sql = format!("SELECT {COLS} FROM attachment WHERE session_id = ?1 ORDER BY id");
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map([session_id], row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn attachment_set_caption(&self, id: i64, caption: Option<&str>) -> Result<()> {
        self.conn().execute("UPDATE attachment SET caption = ?2 WHERE id = ?1", params![id, caption])?;
        Ok(())
    }

    /// Remove the row and its files.
    pub fn attachment_delete(&self, data_dir: &Path, id: i64) -> Result<()> {
        let Some(a) = self.attachment_get(id)? else { return Ok(()) };
        let _ = std::fs::remove_file(data_dir.join(&a.path));
        let _ = std::fs::remove_file(thumb_path(data_dir, id));
        self.conn().execute("DELETE FROM attachment WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Thumbnail as a `data:image/jpeg;base64,...` URL, or None if there is no thumbnail.
    pub fn attachment_thumb_data_url(&self, data_dir: &Path, id: i64) -> Result<Option<String>> {
        let p = thumb_path(data_dir, id);
        if !p.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(p)?;
        Ok(Some(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )))
    }

    /// Sessions that have at least one attachment (for badges in the list).
    pub fn attachment_counts(&self) -> Result<Vec<(i64, i64)>> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT session_id, COUNT(*) FROM attachment WHERE session_id IS NOT NULL GROUP BY session_id",
        )?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// Convert HEIC to JPEG using the OS. macOS ships `sips`; elsewhere we have no decoder.
fn convert_heic_to_jpeg(src: &Path, data_dir: &Path) -> Result<PathBuf> {
    let tmp_dir = data_dir.join(".tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let out = tmp_dir.join(format!(
        "{}-{}.jpg",
        src.file_stem().and_then(|s| s.to_str()).unwrap_or("photo"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    if cfg!(target_os = "macos") {
        let status = std::process::Command::new("sips")
            .args(["-s", "format", "jpeg", "-s", "formatOptions", "90"])
            .arg(src)
            .arg("--out")
            .arg(&out)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if status.success() && out.exists() {
            return Ok(out);
        }
        return Err(Error::Other("HEIC conversion with sips failed".into()));
    }
    // Other platforms: try ImageMagick if present
    for cmd in ["magick", "convert", "heif-convert"] {
        let ok = std::process::Command::new(cmd)
            .arg(src)
            .arg(&out)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok && out.exists() {
            return Ok(out);
        }
    }
    Err(Error::Other(
        "HEIC is not supported on this platform; export the photo as JPEG first".into(),
    ))
}

pub fn thumb_path(data_dir: &Path, id: i64) -> PathBuf {
    data_dir.join(format!("attachments/thumbs/att-{id}.jpg"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Session;

    #[test]
    fn add_thumb_delete() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        let sid = db.session_save(&Session { date: Some("2026-09-01".into()), ..Default::default() }).unwrap();

        // a small PNG source
        let src = dir.path().join("photo.png");
        image::RgbImage::from_fn(900, 600, |x, _| image::Rgb([(x % 255) as u8, 80, 120]))
            .save(&src)
            .unwrap();

        let a = db.attachment_add(dir.path(), sid, &src).unwrap();
        assert_eq!(a.path, format!("attachments/2026/att-{}.png", a.id));
        assert!(dir.path().join(&a.path).exists());
        let thumb = image::open(thumb_path(dir.path(), a.id)).unwrap();
        assert!(thumb.width() <= THUMB_MAX && thumb.height() <= THUMB_MAX);

        let url = db.attachment_thumb_data_url(dir.path(), a.id).unwrap().unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
        assert_eq!(db.attachment_list(sid).unwrap().len(), 1);
        assert_eq!(db.attachment_counts().unwrap(), vec![(sid, 1)]);

        db.attachment_set_caption(a.id, Some("床の花")).unwrap();
        assert_eq!(db.attachment_get(a.id).unwrap().unwrap().caption.as_deref(), Some("床の花"));

        db.attachment_delete(dir.path(), a.id).unwrap();
        assert!(!dir.path().join(&a.path).exists());
        assert!(db.attachment_list(sid).unwrap().is_empty());
    }

    #[test]
    fn deleting_session_cascades_attachments() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        let sid = db.session_save(&Session { date: Some("2026-09-01".into()), ..Default::default() }).unwrap();
        let src = dir.path().join("p.png");
        image::RgbImage::new(10, 10).save(&src).unwrap();
        db.attachment_add(dir.path(), sid, &src).unwrap();
        // re-saving the session (which replaces temae) keeps session-level attachments
        let s = db.session_get(sid).unwrap().unwrap();
        db.session_save(&s).unwrap();
        assert_eq!(db.attachment_list(sid).unwrap().len(), 1);
        db.session_delete(sid).unwrap();
        assert!(db.attachment_list(sid).unwrap().is_empty());
    }
}
