//! Tauri shell: opens the database in the app data dir, installs the bundled pack,
//! and exposes chanote-core operations as commands. All logic lives in chanote-core.

use chanote_core::model::{AxisValues, ResolvedTemae, Session, SessionSummary, Vocab};
use chanote_core::pack::{Pack, URASENKE_YAML};
use chanote_core::attachment::Attachment;
use chanote_core::import::{ImportOptions, ImportPreview, ImportReport};
use chanote_core::record::SessionFilter;
use chanote_core::stats::{CoverageRow, CrossCell, MonthlyRow, StatsFilter, StatsTotals, CROSS_AXES};
use chanote_core::ranking::RankRow;
use chanote_core::stats::ByRow;
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
}

#[tauri::command]
fn app_info(state: State<AppState>) -> CmdResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        db_path: state.o().db_path.display().to_string(),
        profile: state.o().profile.clone(),
        schema_version: state.o().db.schema_version().map_err(e)?,
        self_person_id: state.o().db.self_person_id().map_err(e)?,
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

/// Import a chanote JSON export from `path` (no options: skip duplicates).
#[tauri::command]
fn import_read(state: State<AppState>, path: String) -> CmdResult<ImportReport> {
    let json = std::fs::read_to_string(&path).map_err(e)?;
    state.o().db.import_json(&json).map_err(e)
}

#[tauri::command]
fn import_preview(state: State<AppState>, path: String, options: ImportOptions) -> CmdResult<ImportPreview> {
    let json = std::fs::read_to_string(&path).map_err(e)?;
    state.o().db.import_preview(&json, &options).map_err(e)
}

#[tauri::command]
fn import_apply(state: State<AppState>, path: String, options: ImportOptions) -> CmdResult<ImportReport> {
    let json = std::fs::read_to_string(&path).map_err(e)?;
    state.o().db.import_json_with(&json, &options).map_err(e)
}

/// Full backup (database + photos) as a zip.
#[tauri::command]
fn backup_zip(state: State<AppState>, path: String) -> CmdResult<usize> {
    state.o().db.backup_zip(&state.o().data_dir, std::path::Path::new(&path)).map_err(e)
}

/// Restore from a backup zip. The current database is renamed to
/// `chanote.sqlite.before-restore`; the app must be restarted afterwards (the caller
/// calls `app_restart`).
#[tauri::command]
fn backup_restore(state: State<AppState>, path: String) -> CmdResult<usize> {
    // checkpoint and release the WAL before files are swapped underneath us
    let _ = state.o().db.conn().execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    chanote_core::backup::restore_zip(&state.o().data_dir, std::path::Path::new(&path)).map_err(e)
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
    f.last = id;
    f.save(&state.root).map_err(e)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            let f = Profiles::load(&root)?;
            let open = open_profile(&root, &f.last)?;
            app.manage(AppState { root, open: RwLock::new(open) });
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
            vocab_ranking,
            profile_list,
            profile_create,
            profile_rename,
            profile_set_ask,
            profile_switch,
            export_write,
            import_read,
            import_preview,
            import_apply,
            backup_write,
            backup_zip,
            backup_restore,
            app_restart,
            attachment_add,
            attachment_list,
            attachment_thumb,
            attachment_caption,
            attachment_delete,
            attachment_path,
            attachment_counts,
            attachment_rebuild_thumbs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running chanote");
}
