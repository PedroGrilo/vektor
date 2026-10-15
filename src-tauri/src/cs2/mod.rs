//! STUB — owned by worker "cs2".
pub mod benchmark;
pub mod config;
pub mod launch;
pub mod presets;
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct Cs2Install {}
#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")] pub struct Cs2Config {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct ConfigPreset {}
pub fn detect() -> Result<Cs2Install> { Err(Error::Unsupported) }
/// Steam install root (e.g. C:\Program Files (x86)\Steam), used by tweaks for shader caches.
pub fn steam_path() -> Option<std::path::PathBuf> { None }
/// Full path to game\bin\win64\cs2.exe, used by tweaks (FSO / GPU preference).
pub fn game_exe_path() -> Option<std::path::PathBuf> { None }
