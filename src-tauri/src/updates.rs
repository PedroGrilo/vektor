//! App updates from GitHub releases through the Tauri updater.
//!
//! Each release carries `latest.json` (version, notes, installer URL and its
//! signature). The installer only runs when the signature matches the public key
//! in `tauri.conf.json`, so a tampered download is rejected. Nothing is installed
//! without the user asking for it: [`check`] only reads the manifest.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::error::{Error, Result};

/// Emitted while the installer downloads; payload is [`Progress`].
const PROGRESS_EVENT: &str = "vektor://update-progress";
/// Progress events are throttled to one per this many bytes.
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

fn err(e: tauri_plugin_updater::Error) -> Error {
    Error::msg(format!("Atualizações: {e}"))
}

async fn find(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>> {
    app.updater_builder()
        // Leave the PC as the user had it (resolution, vibrance) before the app exits.
        .on_before_exit(|| {
            let _ = crate::session::stop();
        })
        .build()
        .map_err(err)?
        .check()
        .await
        .map_err(err)
}

pub async fn check(app: &AppHandle) -> Result<UpdateInfo> {
    let current_version = app.package_info().version.to_string();
    Ok(match find(app).await? {
        Some(u) => UpdateInfo {
            current_version,
            available: true,
            // The manifest's own timestamp string, so no date formatting is needed here.
            date: u.raw_json.get("pub_date").and_then(|d| d.as_str()).map(str::to_string),
            version: Some(u.version),
            notes: u.body,
        },
        None => UpdateInfo { current_version, available: false, version: None, notes: None, date: None },
    })
}

/// Download, verify and run the installer. On Windows the process exits as soon
/// as the installer starts, so this only returns on failure.
pub async fn install(app: &AppHandle) -> Result<()> {
    let update = find(app).await?.ok_or_else(|| Error::msg("Já tens a versão mais recente."))?;
    let handle = app.clone();
    let mut downloaded = 0u64;
    let mut reported = 0u64;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let done = total.is_some_and(|t| downloaded >= t);
                if done || downloaded - reported >= PROGRESS_STEP {
                    reported = downloaded;
                    let _ = handle.emit(PROGRESS_EVENT, Progress { downloaded, total });
                }
            },
            || {},
        )
        .await
        .map_err(err)?;
    app.restart()
}
