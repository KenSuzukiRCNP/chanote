//! Full backup: a zip containing `chanote.sqlite` (a consistent VACUUM INTO copy) and the
//! whole `attachments/` folder. Restore unpacks such a zip into the data dir; the app must
//! be restarted afterwards.

use crate::{Db, Error, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

pub const DB_NAME: &str = "chanote.sqlite";

impl Db {
    /// Write a backup zip to `dest`. Returns the number of files written.
    pub fn backup_zip(&self, data_dir: &Path, dest: &Path) -> Result<usize> {
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }
        // consistent snapshot of the database
        let tmp = data_dir.join(".backup-tmp.sqlite");
        let _ = std::fs::remove_file(&tmp);
        self.conn()
            .execute("VACUUM INTO ?1", [tmp.to_string_lossy().as_ref()])?;

        let file = File::create(dest)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let mut n = 0;

        zip.start_file(DB_NAME, opts)?;
        std::io::copy(&mut File::open(&tmp)?, &mut zip)?;
        n += 1;
        let _ = std::fs::remove_file(&tmp);

        let att = data_dir.join("attachments");
        if att.is_dir() {
            // photos are already compressed; store them as-is
            let stored = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .large_file(true);
            for entry in walk(&att)? {
                let rel = entry.strip_prefix(data_dir).map_err(|e| Error::Other(e.to_string()))?;
                let name = rel.to_string_lossy().replace('\\', "/");
                zip.start_file(name, stored)?;
                std::io::copy(&mut File::open(&entry)?, &mut zip)?;
                n += 1;
            }
        }
        zip.finish()?;
        Ok(n)
    }
}

/// Unpack a backup zip into `data_dir`, replacing the database and attachments.
/// Call this with the database CLOSED (the app restarts after restore).
pub fn restore_zip(data_dir: &Path, src: &Path) -> Result<usize> {
    let mut archive = zip::ZipArchive::new(File::open(src)?)?;
    if archive.by_name(DB_NAME).is_err() {
        return Err(Error::Other(format!("{} not found in archive", DB_NAME)));
    }
    // keep the current db aside until the restore succeeds
    let db_path = data_dir.join(DB_NAME);
    let keep = data_dir.join("chanote.sqlite.before-restore");
    if db_path.exists() {
        let _ = std::fs::remove_file(&keep);
        std::fs::rename(&db_path, &keep)?;
    }
    for suffix in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(data_dir.join(format!("{DB_NAME}{suffix}")));
    }
    let mut n = 0;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        let Some(rel) = f.enclosed_name() else { continue };
        if f.is_dir() {
            continue;
        }
        let out = data_dir.join(rel);
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        File::create(&out)?.write_all(&buf)?;
        n += 1;
    }
    Ok(n)
}

fn walk(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            out.extend(walk(&p)?);
        } else if p.file_name().and_then(|n| n.to_str()) != Some(".DS_Store") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Other(format!("zip: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Session;

    #[test]
    fn backup_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        let db = Db::open(&data.join(DB_NAME)).unwrap();
        let sid = db.session_save(&Session { date: Some("2026-09-01".into()), ..Default::default() }).unwrap();
        let src = dir.path().join("p.png");
        image::RgbImage::new(20, 20).save(&src).unwrap();
        db.attachment_add(&data, sid, &src).unwrap();

        let zip_path = dir.path().join("backup.zip");
        let n = db.backup_zip(&data, &zip_path).unwrap();
        assert_eq!(n, 3); // sqlite + photo + thumb
        drop(db);

        let data2 = dir.path().join("data2");
        std::fs::create_dir_all(&data2).unwrap();
        let n = restore_zip(&data2, &zip_path).unwrap();
        assert_eq!(n, 3);
        let db2 = Db::open(&data2.join(DB_NAME)).unwrap();
        assert_eq!(db2.session_list(10, 0).unwrap().len(), 1);
        let a = db2.attachment_list(sid).unwrap();
        assert_eq!(a.len(), 1);
        assert!(data2.join(&a[0].path).exists());
    }
}
