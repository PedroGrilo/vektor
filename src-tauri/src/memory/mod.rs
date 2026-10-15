//! STUB — owned by worker "memory".
pub mod service;
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};
pub const SERVICE_ARG: &str = "--memory-service";
#[derive(Serialize, Deserialize)] #[serde(rename_all = "camelCase")] pub struct MemoryConfig {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct MemoryStatus {}
pub fn status() -> Result<MemoryStatus> { Err(Error::Unsupported) }
pub fn set_config(_c: &MemoryConfig) -> Result<MemoryStatus> { Err(Error::Unsupported) }
pub fn install_service() -> Result<MemoryStatus> { Err(Error::Unsupported) }
pub fn uninstall_service() -> Result<MemoryStatus> { Err(Error::Unsupported) }
pub fn purge_now() -> Result<MemoryStatus> { Err(Error::Unsupported) }
