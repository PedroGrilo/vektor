use crate::error::{Error, Result};
use super::DisplayInfo;
pub fn list() -> Result<Vec<DisplayInfo>> { Err(Error::Unsupported) }
/// Apply a mode; returns the previous one.
pub fn set_mode(_device: &str, _w: u32, _h: u32, _hz: u32) -> Result<super::DisplayMode> { Err(Error::Unsupported) }
