//! STUB — owned by worker "session".
pub mod affinity;
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(rename_all = "camelCase")] pub struct SessionConfig {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct SessionStatus {}
pub fn status() -> Result<SessionStatus> { Err(Error::Unsupported) }
pub fn set_config(_c: SessionConfig) -> Result<SessionStatus> { Err(Error::Unsupported) }
pub fn stop() -> Result<SessionStatus> { Err(Error::Unsupported) }
/// Start the background watcher thread (called once from lib.rs setup).
pub fn spawn_watcher() {}
/// Revert a session left over from a crash (called at startup).
pub fn recover() {}
