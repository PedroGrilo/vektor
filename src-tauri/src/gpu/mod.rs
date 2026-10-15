//! STUB — owned by worker "gpu/ryzen".
pub mod display;
pub mod nvidia;
pub mod vibrance_watcher;
use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct GpuStatus {}
pub fn status() -> Result<GpuStatus> { Err(Error::Unsupported) }
pub fn set_vibrance_all(_level: i32) -> Result<()> { Err(Error::Unsupported) }
pub fn restore_vibrance(_display: &str, _previous: i32) -> Result<()> { Err(Error::Unsupported) }
