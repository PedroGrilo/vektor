//! STUB — NVIDIA global profile via NVIDIA Profile Inspector.
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
#[derive(Deserialize)] #[serde(rename_all = "camelCase")] pub struct NvGlobalOptions {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct NvidiaStatus {}
pub fn status(_app: &tauri::AppHandle) -> Result<NvidiaStatus> { Err(Error::Unsupported) }
pub fn apply_global(_app: &tauri::AppHandle, _o: &NvGlobalOptions) -> Result<NvidiaStatus> { Err(Error::Unsupported) }
pub fn revert_global(_app: &tauri::AppHandle) -> Result<NvidiaStatus> { Err(Error::Unsupported) }
pub fn restore_base_profile(_snapshot: &Path) -> Result<()> { Err(Error::Unsupported) }
