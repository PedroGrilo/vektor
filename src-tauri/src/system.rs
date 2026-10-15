//! STUB — owned by worker "tweaks/system".
use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct SystemInfo {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct LiveStats {}
pub fn info() -> Result<SystemInfo> { Err(Error::Unsupported) }
pub fn live_stats() -> Result<LiveStats> { Err(Error::Unsupported) }
