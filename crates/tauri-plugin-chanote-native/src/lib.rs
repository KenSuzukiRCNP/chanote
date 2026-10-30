//! Native helpers for chanote on iOS (no-ops elsewhere).
//!
//! - `pick_folder`: the system folder picker (Files app UI). Returns the folder's path and a
//!   security-scoped bookmark; access lasts while the app runs.
//! - `restore_access`: turn a saved bookmark back into a path with access (call on launch).
//! - `ensure_downloaded`: ask iCloud to download files under a folder that are only in the
//!   cloud; returns how many are still pending.
//!
//! The app does the actual file I/O with `std::fs` while access is held.

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_chanote_native);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Folder {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub bookmark: Option<String>,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadStatus {
    pub pending: u32,
}

pub struct Native<R: Runtime> {
    #[cfg(target_os = "ios")]
    handle: tauri::plugin::PluginHandle<R>,
    _r: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> Native<R> {
    pub fn supported(&self) -> bool {
        cfg!(target_os = "ios")
    }

    /// Blocks until the user picks or cancels — call from a background thread.
    pub fn pick_folder(&self) -> Result<Folder, String> {
        #[cfg(target_os = "ios")]
        {
            return self.handle.run_mobile_plugin("pickFolder", ()).map_err(|e| e.to_string());
        }
        #[allow(unreachable_code)]
        Err("folder picking via the native plugin is only needed on iOS".into())
    }

    pub fn restore_access(&self, bookmark: &str) -> Result<Folder, String> {
        #[cfg(target_os = "ios")]
        {
            return self
                .handle
                .run_mobile_plugin("restoreAccess", serde_json::json!({ "bookmark": bookmark }))
                .map_err(|e| e.to_string());
        }
        #[allow(unreachable_code)]
        {
            let _ = bookmark;
            Err("only on iOS".into())
        }
    }

    pub fn ensure_downloaded(&self, path: &str) -> Result<DownloadStatus, String> {
        #[cfg(target_os = "ios")]
        {
            return self
                .handle
                .run_mobile_plugin("ensureDownloaded", serde_json::json!({ "path": path }))
                .map_err(|e| e.to_string());
        }
        #[allow(unreachable_code)]
        {
            let _ = path;
            Ok(DownloadStatus { pending: 0 })
        }
    }
}

pub trait NativeExt<R: Runtime> {
    fn native(&self) -> &Native<R>;
}

impl<R: Runtime, T: Manager<R>> NativeExt<R> for T {
    fn native(&self) -> &Native<R> {
        self.state::<Native<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("chanote-native")
        .setup(|app, api| {
            #[cfg(target_os = "ios")]
            let handle = api.register_ios_plugin(init_plugin_chanote_native)?;
            #[cfg(not(target_os = "ios"))]
            let _ = api;
            app.manage(Native::<R> {
                #[cfg(target_os = "ios")]
                handle,
                _r: std::marker::PhantomData,
            });
            Ok(())
        })
        .build()
}
