//! Tauri shell: opens the database in the app data dir, installs the bundled pack,
//! and exposes chanote-core operations as commands. All logic lives in chanote-core.

use chanote_core::model::{AxisValues, ResolvedTemae, Session, SessionSummary, Vocab};
use chanote_core::pack::{Pack, URASENKE_YAML};
use chanote_core::attachment::Attachment;
use chanote_core::import::{ImportOptions, ImportPreview, ImportReport};
use chanote_core::record::SessionFilter;
use chanote_core::stats::{CoverageRow, CrossCell, MonthlyRow, StatsFilter, StatsTotals, CROSS_AXES};
use chanote_core::ranking::RankRow;
use chanote_core::stats::{ByRow, EventRow};
use chanote_core::Db;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard};
use tauri::{Manager, State};

mod profiles;
use profiles::Profiles;

/// The currently open profile.
struct Open {
    profile: String,
    db: Db,
    db_path: PathBuf,
    /// directory holding chanote.sqlite and attachments/
    data_dir: PathBuf,
}

struct AppState {
    /// app data root: profiles.json lives here; the default profile's data too
    root: PathBuf,
    open: RwLock<Open>,
    /// iOS: sync folder path resolved from its bookmark (access is held for the session)
    sync_parent: std::sync::Mutex<Option<PathBuf>>,
}

impl AppState {
    fn o(&self) -> RwLockReadGuard<'_, Open> {
        self.open.read().expect("state lock poisoned")
    }
}

/// Errors cross the IPC boundary as plain strings.
type CmdResult<T> = Result<T, String>;

fn e(err: impl std::fmt::Display) -> String {
    err.to_string()
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    db_path: String,
    schema_version: i64,
    self_person_id: String,
    profile: String,
    /// "macos" | "windows" | "linux" | "ios" | "android"
    platform: String,
    mobile: bool,
    /// Where exports and backups are written on mobile (the app's Documents folder,
    /// visible in the Files app on iOS). Empty on desktop (save dialog instead).
    exports_dir: String,
}

const MOBILE: bool = cfg!(any(target_os = "ios", target_os = "android"));

/// Folder for files the user wants to get out of the app on mobile.
fn exports_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if !MOBILE {
        return None;
    }
    // iOS: <app>/Documents, exposed in Files via UIFileSharingEnabled (Info.ios.plist).
    // Android: the app's own files dir; the share/save dialog copies from there.
    let dir = app.path().document_dir().ok().or_else(|| app.path().app_data_dir().ok())?;
    let dir = if cfg!(target_os = "android") { dir.join("exports") } else { dir };
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

#[tauri::command]
fn app_info(app: tauri::AppHandle, state: State<AppState>) -> CmdResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        db_path: state.o().db_path.display().to_string(),
        profile: state.o().profile.clone(),
        schema_version: state.o().db.schema_version().map_err(e)?,
        self_person_id: state.o().db.self_person_id().map_err(e)?,
        platform: std::env::consts::OS.into(),
        mobile: MOBILE,
        exports_dir: exports_dir(&app).map(|d| d.display().to_string()).unwrap_or_default(),
    })
}

// ---- settings ---------------------------------------------------------------

#[tauri::command]
fn setting_get(state: State<AppState>, key: String) -> CmdResult<Option<String>> {
    state.o().db.get_setting(&key).map_err(e)
}

#[tauri::command]
fn setting_set(state: State<AppState>, key: String, value: String) -> CmdResult<()> {
    state.o().db.set_setting(&key, &value).map_err(e)
}

// ---- vocabulary -------------------------------------------------------------

#[tauri::command]
fn vocab_list(state: State<AppState>, kind: String, include_archived: bool) -> CmdResult<Vec<Vocab>> {
    state.o().db.vocab_list(&kind, include_archived).map_err(e)
}

#[tauri::command]
fn vocab_upsert(state: State<AppState>, vocab: Vocab) -> CmdResult<()> {
    state.o().db.vocab_upsert(&vocab).map_err(e)
}

#[tauri::command]
fn vocab_archive(state: State<AppState>, id: String, archived: bool) -> CmdResult<()> {
    state.o().db.vocab_archive(&id, archived).map_err(e)
}

#[tauri::command]
fn pack_reset_row(state: State<AppState>, id: String) -> CmdResult<bool> {
    let pack = Pack::from_yaml(URASENKE_YAML).map_err(e)?;
    state.o().db.pack_reset_row(&pack, &id).map_err(e)
}

#[tauri::command]
fn vocab_delete(state: State<AppState>, id: String) -> CmdResult<()> {
    state.o().db.vocab_delete(&id).map_err(e)
}

#[tauri::command]
fn vocab_usage(state: State<AppState>, id: String) -> CmdResult<i64> {
    state.o().db.vocab_usage(&id).map_err(e)
}

#[tauri::command]
fn resolve_temae(state: State<AppState>, id: String) -> CmdResult<ResolvedTemae> {
    state.o().db.resolve_temae(&id).map_err(e)
}

#[tauri::command]
fn temae_defaults(state: State<AppState>, id: String) -> CmdResult<AxisValues> {
    state.o().db.temae_defaults(&id).map_err(e)
}

// ---- records ----------------------------------------------------------------

#[tauri::command]
fn session_list(state: State<AppState>, limit: i64, offset: i64) -> CmdResult<Vec<SessionSummary>> {
    state.o().db.session_list(limit, offset).map_err(e)
}

#[tauri::command]
fn session_search(state: State<AppState>, filter: SessionFilter, limit: i64, offset: i64) -> CmdResult<Vec<SessionSummary>> {
    state.o().db.session_search(&filter, limit, offset).map_err(e)
}

#[tauri::command]
fn session_get(state: State<AppState>, id: i64) -> CmdResult<Option<Session>> {
    state.o().db.session_get(id).map_err(e)
}

#[tauri::command]
fn session_save(state: State<AppState>, session: Session) -> CmdResult<i64> {
    state.o().db.session_save(&session).map_err(e)
}

#[tauri::command]
fn session_delete(state: State<AppState>, id: i64) -> CmdResult<()> {
    state.o().db.session_delete(id).map_err(e)
}

// ---- stats ------------------------------------------------------------------

#[tauri::command]
fn stats_coverage(state: State<AppState>, filter: StatsFilter) -> CmdResult<Vec<CoverageRow>> {
    state.o().db.stats_coverage(&filter).map_err(e)
}

#[tauri::command]
fn stats_totals(state: State<AppState>, filter: StatsFilter) -> CmdResult<StatsTotals> {
    state.o().db.stats_totals(&filter).map_err(e)
}

#[tauri::command]
fn stats_monthly(state: State<AppState>, filter: StatsFilter) -> CmdResult<Vec<MonthlyRow>> {
    state.o().db.stats_monthly(&filter).map_err(e)
}

#[tauri::command]
fn stats_cross(
    state: State<AppState>,
    row_axis: String,
    col_axis: String,
    host_only: bool,
    filter: StatsFilter,
) -> CmdResult<Vec<CrossCell>> {
    state.o().db.stats_cross(&row_axis, &col_axis, host_only, &filter).map_err(e)
}

#[tauri::command]
fn stats_by(state: State<AppState>, dimension: String, filter: StatsFilter) -> CmdResult<Vec<ByRow>> {
    state.o().db.stats_by(&dimension, &filter).map_err(e)
}

#[tauri::command]
fn stats_events(state: State<AppState>, filter: StatsFilter) -> CmdResult<Vec<EventRow>> {
    state.o().db.stats_events(&filter).map_err(e)
}

#[tauri::command]
fn vocab_ranking(state: State<AppState>, kind: String) -> CmdResult<Vec<RankRow>> {
    state.o().db.vocab_ranking(&kind).map_err(e)
}

#[tauri::command]
fn stats_axes() -> Vec<String> {
    CROSS_AXES.iter().map(|(n, _)| n.to_string()).collect()
}

// ---- attachments --------------------------------------------------------------

#[tauri::command]
fn attachment_add(state: State<AppState>, session_id: i64, paths: Vec<String>) -> CmdResult<Vec<Attachment>> {
    let mut out = Vec::new();
    for p in paths {
        out.push(
            state.o().db
                .attachment_add(&state.o().data_dir, session_id, std::path::Path::new(&p))
                .map_err(e)?,
        );
    }
    Ok(out)
}

/// Mobile variant: the picked file arrives as the raw request body (the WebView read
/// it through the fs plugin), with the session id and file name in headers.
#[tauri::command]
fn attachment_add_bytes(state: State<AppState>, request: tauri::ipc::Request<'_>) -> CmdResult<Attachment> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected a binary body".into());
    };
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).map(|v| v.to_string());
    let session_id: i64 = header("x-session-id").and_then(|v| v.parse().ok()).ok_or("missing x-session-id")?;
    let name = header("x-file-name").unwrap_or_else(|| "photo.jpg".into());
    let name = percent_decode(&name);
    let o = state.o();
    let tmp = o.data_dir.join(".tmp");
    std::fs::create_dir_all(&tmp).map_err(e)?;
    let path = tmp.join(format!("in-{}-{}", std::process::id(), name.replace(['/', '\\'], "_")));
    std::fs::write(&path, bytes).map_err(e)?;
    let res = o.db.attachment_add(&o.data_dir, session_id, &path).map_err(e);
    let _ = std::fs::remove_file(&path);
    res
}

/// Minimal percent-decoding for header values (file names with non-ASCII characters).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
fn attachment_list(state: State<AppState>, session_id: i64) -> CmdResult<Vec<Attachment>> {
    state.o().db.attachment_list(session_id).map_err(e)
}

#[tauri::command]
fn attachment_thumb(state: State<AppState>, id: i64) -> CmdResult<Option<String>> {
    state.o().db.attachment_thumb_data_url(&state.o().data_dir, id).map_err(e)
}

#[tauri::command]
fn attachment_caption(state: State<AppState>, id: i64, caption: Option<String>) -> CmdResult<()> {
    state.o().db.attachment_set_caption(id, caption.as_deref()).map_err(e)
}

#[tauri::command]
fn attachment_delete(state: State<AppState>, id: i64) -> CmdResult<()> {
    state.o().db.attachment_delete(&state.o().data_dir, id).map_err(e)
}

/// Absolute path of the original file (for opening in the system viewer).
#[tauri::command]
fn attachment_path(state: State<AppState>, id: i64) -> CmdResult<Option<String>> {
    Ok(state.o().db
        .attachment_get(id)
        .map_err(e)?
        .map(|a| state.o().data_dir.join(a.path).display().to_string()))
}

#[tauri::command]
fn attachment_rebuild_thumbs(state: State<AppState>) -> CmdResult<(usize, usize)> {
    let o = state.o();
    o.db.attachment_rebuild_thumbs(&o.data_dir).map_err(e)
}

#[tauri::command]
fn attachment_counts(state: State<AppState>) -> CmdResult<Vec<(i64, i64)>> {
    state.o().db.attachment_counts().map_err(e)
}

// ---- export -----------------------------------------------------------------

/// Write an export to `path` (chosen by the user through the save dialog on the JS side).
/// `format` is "json" or "csv". Returns the number of bytes written.
#[tauri::command]
fn export_write(state: State<AppState>, path: String, format: String) -> CmdResult<usize> {
    let body = match format.as_str() {
        "json" => state.o().db.export_json().map_err(e)?,
        "csv" => {
            // UTF-8 BOM so Excel on macOS/Windows opens Japanese text correctly
            let mut s = String::from("\u{feff}");
            s.push_str(&state.o().db.export_csv().map_err(e)?);
            s
        }
        other => return Err(format!("unknown export format {other}")),
    };
    std::fs::write(&path, body.as_bytes()).map_err(e)?;
    Ok(body.len())
}

/// Export as text (mobile: the WebView writes it where the OS lets it, e.g. through
/// the Android save dialog).
#[tauri::command]
fn export_text(state: State<AppState>, format: String) -> CmdResult<String> {
    match format.as_str() {
        "json" => state.o().db.export_json().map_err(e),
        "csv" => Ok(format!("\u{feff}{}", state.o().db.export_csv().map_err(e)?)),
        other => Err(format!("unknown export format {other}")),
    }
}

/// Mobile: write an export into the exports folder (iOS: Documents → Files app).
/// Returns the full path.
#[tauri::command]
fn export_to_documents(app: tauri::AppHandle, state: State<AppState>, name: String, format: String) -> CmdResult<String> {
    let dir = exports_dir(&app).ok_or("no exports folder on this platform")?;
    let path = dir.join(name);
    export_write(state, path.display().to_string(), format)?;
    Ok(path.display().to_string())
}

/// Source of an import: a file path (desktop) or the JSON text itself (mobile, read by
/// the WebView through the fs plugin).
#[derive(Deserialize)]
struct ImportSource {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

impl ImportSource {
    fn json(&self) -> CmdResult<String> {
        if let Some(t) = &self.text {
            return Ok(t.clone());
        }
        let p = self.path.as_ref().ok_or("import source has neither path nor text")?;
        std::fs::read_to_string(p).map_err(e)
    }
}

/// Import a chanote JSON export (no options: skip duplicates).
#[tauri::command]
fn import_read(state: State<AppState>, source: ImportSource) -> CmdResult<ImportReport> {
    state.o().db.import_json(&source.json()?).map_err(e)
}

#[tauri::command]
fn import_preview(state: State<AppState>, source: ImportSource, options: ImportOptions) -> CmdResult<ImportPreview> {
    state.o().db.import_preview(&source.json()?, &options).map_err(e)
}

#[tauri::command]
fn import_apply(state: State<AppState>, source: ImportSource, options: ImportOptions) -> CmdResult<ImportReport> {
    state.o().db.import_json_with(&source.json()?, &options).map_err(e)
}

/// Full backup (database + photos) as a zip.
#[tauri::command]
fn backup_zip(state: State<AppState>, path: String) -> CmdResult<usize> {
    state.o().db.backup_zip(&state.o().data_dir, std::path::Path::new(&path)).map_err(e)
}

/// Mobile: backup zip into the exports folder. Returns (path, file count).
#[tauri::command]
fn backup_zip_to_documents(app: tauri::AppHandle, state: State<AppState>, name: String) -> CmdResult<(String, usize)> {
    let dir = exports_dir(&app).ok_or("no exports folder on this platform")?;
    let path = dir.join(name);
    let n = state.o().db.backup_zip(&state.o().data_dir, &path).map_err(e)?;
    Ok((path.display().to_string(), n))
}

/// Mobile: the backup zip as raw bytes (Android writes it through the save dialog).
#[tauri::command]
fn backup_zip_bytes(state: State<AppState>) -> CmdResult<tauri::ipc::Response> {
    let o = state.o();
    let tmp = o.data_dir.join(".tmp");
    std::fs::create_dir_all(&tmp).map_err(e)?;
    let path = tmp.join(format!("backup-{}.zip", std::process::id()));
    o.db.backup_zip(&o.data_dir, &path).map_err(e)?;
    let bytes = std::fs::read(&path).map_err(e)?;
    let _ = std::fs::remove_file(&path);
    Ok(tauri::ipc::Response::new(bytes))
}

/// Mobile: restore from a zip delivered as the raw request body.
#[tauri::command]
fn backup_restore_bytes(state: State<AppState>, request: tauri::ipc::Request<'_>) -> CmdResult<usize> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected a binary body".into());
    };
    let path = {
        let o = state.o();
        let tmp = o.data_dir.join(".tmp");
        std::fs::create_dir_all(&tmp).map_err(e)?;
        let path = tmp.join(format!("restore-{}.zip", std::process::id()));
        std::fs::write(&path, bytes).map_err(e)?;
        path
    };
    let res = backup_restore(state, path.display().to_string());
    let _ = std::fs::remove_file(&path);
    res
}

/// Restore from a backup zip. The current database is renamed to
/// `chanote.sqlite.before-restore` and the profile is reopened in place (no process
/// restart — iOS cannot relaunch an app); the frontend reloads afterwards.
#[tauri::command]
fn backup_restore(state: State<AppState>, path: String) -> CmdResult<usize> {
    let (profile, data_dir) = {
        let o = state.o();
        // checkpoint and release the WAL before files are swapped underneath us
        let _ = o.db.conn().execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        (o.profile.clone(), o.data_dir.clone())
    };
    // close the live connection while the files are replaced
    let placeholder = Open {
        profile: profile.clone(),
        db: Db::open_in_memory().map_err(e)?,
        db_path: PathBuf::new(),
        data_dir: data_dir.clone(),
    };
    let old = {
        let mut guard = state.open.write().map_err(|_| "state lock poisoned".to_string())?;
        std::mem::replace(&mut *guard, placeholder)
    };
    drop(old);
    let n = chanote_core::backup::restore_zip(&data_dir, std::path::Path::new(&path)).map_err(e);
    let reopened = open_profile(&state.root, &profile).map_err(e)?;
    // the restored database may carry another device's sync identity: never reuse it
    if n.is_ok() {
        reopened.db.sync_reset().map_err(e)?;
    }
    {
        let mut guard = state.open.write().map_err(|_| "state lock poisoned".to_string())?;
        *guard = reopened;
    }
    *state.sync_parent.lock().unwrap() = None;
    n
}

#[tauri::command]
fn app_restart(app: tauri::AppHandle) {
    app.restart();
}

/// Copy the live database to `path` (backup). Uses SQLite's VACUUM INTO so the copy is
/// consistent even while the app is running.
#[tauri::command]
fn backup_write(state: State<AppState>, path: String) -> CmdResult<()> {
    if std::path::Path::new(&path).exists() {
        std::fs::remove_file(&path).map_err(e)?;
    }
    state.o().db
        .conn()
        .execute("VACUUM INTO ?1", [&path])
        .map_err(e)?;
    Ok(())
}

// ---- sync (folder of change logs; see chanote-core/src/sync.rs) ------------------

use chanote_core::sync::{Conflict, DiaryInfo, SyncReport, SyncStatus};
use tauri_plugin_chanote_native::NativeExt;

#[derive(Serialize)]
struct SyncInfo {
    #[serde(flatten)]
    status: SyncStatus,
    /// the folder the user chose (display only)
    folder: Option<String>,
    /// false on Android (no shared folder support yet)
    supported: bool,
}

const SYNC_SUPPORTED: bool = !cfg!(target_os = "android");

fn device_label() -> String {
    match std::env::consts::OS {
        "macos" => "Mac".into(),
        "ios" => "iPhone/iPad".into(),
        "windows" => "Windows".into(),
        "linux" => "Linux".into(),
        other => other.into(),
    }
}

/// The parent folder of `chanote-sync/` for the open profile. On iOS this re-opens access
/// through the saved bookmark (once per launch).
fn sync_parent(app: &tauri::AppHandle, state: &AppState) -> Result<PathBuf, String> {
    if let Some(p) = state.sync_parent.lock().unwrap().clone() {
        return Ok(p);
    }
    let db_setting = |k: &str| state.o().db.get_setting(k).map_err(e);
    let path = if cfg!(target_os = "ios") {
        let bookmark = db_setting("sync_bookmark")?.ok_or("no sync folder chosen")?;
        let f = app.native().restore_access(&bookmark)?;
        if let Some(b) = f.bookmark.as_deref().filter(|b| *b != bookmark) {
            state.o().db.set_setting("sync_bookmark", b).map_err(e)?;
        }
        PathBuf::from(f.path.ok_or("sync folder unavailable")?)
    } else {
        PathBuf::from(db_setting("sync_parent")?.ok_or("no sync folder chosen")?)
    };
    *state.sync_parent.lock().unwrap() = Some(path.clone());
    Ok(path)
}

fn sync_info(state: &AppState) -> CmdResult<SyncInfo> {
    let o = state.o();
    Ok(SyncInfo {
        status: o.db.sync_status().map_err(e)?,
        folder: o.db.get_setting("sync_parent").map_err(e)?,
        supported: SYNC_SUPPORTED,
    })
}

#[tauri::command]
fn sync_status(state: State<AppState>) -> CmdResult<SyncInfo> {
    sync_info(&state)
}

/// Let the user choose the parent folder. Desktop: `path` comes from the JS folder dialog.
/// iOS: `path` is ignored and the native picker is shown. Returns the diaries found there.
#[tauri::command]
async fn sync_choose_folder(app: tauri::AppHandle, path: Option<String>) -> CmdResult<(String, Vec<DiaryInfo>)> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let parent = if cfg!(target_os = "ios") {
            let f = app.native().pick_folder()?;
            if f.cancelled {
                return Err("cancelled".to_string());
            }
            let o = state.o();
            o.db.set_setting("sync_bookmark", f.bookmark.as_deref().unwrap_or_default()).map_err(e)?;
            PathBuf::from(f.path.ok_or("no folder")?)
        } else {
            PathBuf::from(path.ok_or("no folder")?)
        };
        state.o().db.set_setting("sync_parent", &parent.display().to_string()).map_err(e)?;
        *state.sync_parent.lock().unwrap() = Some(parent.clone());
        let list = chanote_core::sync::list_diaries(&parent).map_err(e)?;
        Ok((parent.display().to_string(), list))
    })
    .await
    .map_err(e)?
}

/// Create a new diary (`id` = None, `name` given) or join an existing one in the chosen folder.
#[tauri::command]
async fn sync_setup(app: tauri::AppHandle, id: Option<String>, name: Option<String>) -> CmdResult<SyncReport> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let parent = sync_parent(&app, &state)?;
        let o = state.o();
        match id {
            Some(id) => o.db.sync_join(&parent, &id, &o.data_dir, &device_label()).map_err(e),
            None => o
                .db
                .sync_create(&parent, name.as_deref().unwrap_or("chanote"), &o.data_dir, &device_label())
                .map_err(e),
        }
    })
    .await
    .map_err(e)?
}

#[tauri::command]
async fn sync_now(app: tauri::AppHandle) -> CmdResult<SyncReport> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let parent = sync_parent(&app, &state)?;
        let o = state.o();
        if cfg!(target_os = "ios") {
            if let Some(id) = o.db.get_setting("sync_diary").map_err(e)? {
                // ask iCloud for anything that is only in the cloud; it arrives by the next round
                let _ = app.native().ensure_downloaded(&chanote_core::sync::diary_dir(&parent, &id).display().to_string());
            }
        }
        o.db.sync_run(&parent, &o.data_dir).map_err(e)
    })
    .await
    .map_err(e)?
}

#[tauri::command]
fn sync_disable(state: State<AppState>) -> CmdResult<()> {
    let o = state.o();
    o.db.sync_reset().map_err(e)?;
    let _ = o.db.conn().execute("DELETE FROM setting WHERE key IN ('sync_parent','sync_bookmark')", []);
    *state.sync_parent.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
fn sync_conflicts(state: State<AppState>) -> CmdResult<Vec<Conflict>> {
    state.o().db.sync_conflicts().map_err(e)
}

#[tauri::command]
fn sync_conflict_resolve(state: State<AppState>, id: i64, restore: bool) -> CmdResult<()> {
    let o = state.o();
    o.db.sync_conflict_resolve(id, restore, &o.data_dir).map_err(e)
}

// ---- setup ------------------------------------------------------------------

fn open_profile(root: &Path, profile: &str) -> Result<Open, Box<dyn std::error::Error>> {
    // Override for development / portable use: CHANOTE_DB=/path/to/file.sqlite (ignores profiles)
    let db_path = match std::env::var_os("CHANOTE_DB") {
        Some(p) => PathBuf::from(p),
        None => profiles::data_dir_for(root, profile).join(chanote_core::backup::DB_NAME),
    };
    let data_dir = db_path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&data_dir)?;
    let db = Db::open(&db_path)?;
    let pack = Pack::from_yaml(URASENKE_YAML)?;
    db.pack_install(&pack)?;
    db.self_person_id()?;
    Ok(Open { profile: profile.to_string(), db, db_path, data_dir })
}

// ---- profiles (several people sharing one computer account) -------------------

#[derive(Serialize)]
struct ProfileInfo {
    profiles: Vec<profiles::Profile>,
    current: String,
    ask_on_start: bool,
}

#[tauri::command]
fn profile_list(state: State<AppState>) -> CmdResult<ProfileInfo> {
    let f = Profiles::load(&state.root).map_err(e)?;
    Ok(ProfileInfo { profiles: f.profiles, current: state.o().profile.clone(), ask_on_start: f.ask_on_start })
}

#[derive(Deserialize)]
struct ProfileCreate {
    name: String,
}

#[tauri::command]
fn profile_create(state: State<AppState>, input: ProfileCreate) -> CmdResult<profiles::Profile> {
    let mut f = Profiles::load(&state.root).map_err(e)?;
    let p = f.add(&input.name);
    f.save(&state.root).map_err(e)?;
    Ok(p)
}

#[tauri::command]
fn profile_rename(state: State<AppState>, id: String, name: String) -> CmdResult<()> {
    let mut f = Profiles::load(&state.root).map_err(e)?;
    f.rename(&id, &name).map_err(e)?;
    f.save(&state.root).map_err(e)
}

#[tauri::command]
fn profile_set_ask(state: State<AppState>, ask: bool) -> CmdResult<()> {
    let mut f = Profiles::load(&state.root).map_err(e)?;
    f.ask_on_start = ask;
    f.save(&state.root).map_err(e)
}

/// Switch the open database to another profile. The frontend reloads afterwards.
#[tauri::command]
fn profile_switch(state: State<AppState>, id: String) -> CmdResult<()> {
    let mut f = Profiles::load(&state.root).map_err(e)?;
    if !f.profiles.iter().any(|p| p.id == id) {
        return Err(format!("unknown profile {id}"));
    }
    let new = open_profile(&state.root, &id).map_err(e)?;
    {
        let mut guard = state.open.write().map_err(|_| "state lock poisoned".to_string())?;
        let _ = guard.db.conn().execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        *guard = new;
    }
    *state.sync_parent.lock().unwrap() = None;
    f.last = id;
    f.save(&state.root).map_err(e)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_chanote_native::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            let f = Profiles::load(&root)?;
            let open = open_profile(&root, &f.last)?;
            app.manage(AppState { root, open: RwLock::new(open), sync_parent: std::sync::Mutex::new(None) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            setting_get,
            setting_set,
            vocab_list,
            vocab_upsert,
            vocab_archive,
            vocab_delete,
            vocab_usage,
            pack_reset_row,
            resolve_temae,
            temae_defaults,
            session_list,
            session_get,
            session_search,
            session_save,
            session_delete,
            stats_coverage,
            stats_totals,
            stats_monthly,
            stats_cross,
            stats_axes,
            stats_by,
            stats_events,
            vocab_ranking,
            profile_list,
            profile_create,
            profile_rename,
            profile_set_ask,
            profile_switch,
            export_write,
            export_text,
            export_to_documents,
            import_read,
            import_preview,
            import_apply,
            backup_write,
            backup_zip,
            backup_zip_to_documents,
            backup_zip_bytes,
            backup_restore,
            backup_restore_bytes,
            app_restart,
            attachment_add,
            attachment_add_bytes,
            attachment_list,
            attachment_thumb,
            attachment_caption,
            attachment_delete,
            attachment_path,
            attachment_counts,
            attachment_rebuild_thumbs,
            sync_status,
            sync_choose_folder,
            sync_setup,
            sync_now,
            sync_disable,
            sync_conflicts,
            sync_conflict_resolve,
        ])
        .run(tauri::generate_context!())
        .expect("error while running chanote");
}
