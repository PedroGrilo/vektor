use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct VibranceInfo {}
pub fn info() -> Result<VibranceInfo> { Err(Error::Unsupported) }
pub fn preview(_level: i32) -> Result<VibranceInfo> { Err(Error::Unsupported) }
pub fn reset() -> Result<VibranceInfo> { Err(Error::Unsupported) }
/// Set vibrance (0..100) on `device` (empty = primary).
pub fn set_level(_device: &str, _level: i32) -> Result<()> { Err(Error::Unsupported) }
/// Current level (0..100) on `device` (empty = primary).
pub fn get_level(_device: &str) -> Result<i32> { Err(Error::Unsupported) }
