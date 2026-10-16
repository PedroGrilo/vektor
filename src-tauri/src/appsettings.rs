//! STUB — owned by worker "extras".
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct AppSettings { pub autostart: bool, pub start_minimized: bool, pub close_to_tray: bool }
/// Settings file only (no system queries) — cheap, used by lib.rs at startup.
pub fn load() -> AppSettings { AppSettings::default() }
/// Settings with `autostart` reflecting the real scheduled task.
pub fn load_checked() -> Result<AppSettings> { Err(Error::Unsupported) }
pub fn save(_s: AppSettings) -> Result<AppSettings> { Err(Error::Unsupported) }
