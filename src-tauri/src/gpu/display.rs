use crate::error::{Error, Result};
/// Returns the previous mode.
pub fn set_mode(_device: &str, _w: u32, _h: u32, _hz: u32) -> Result<(u32, u32, u32)> { Err(Error::Unsupported) }
pub fn apply_mode_journaled(_device: &str, _w: u32, _h: u32, _hz: u32) -> Result<()> { Err(Error::Unsupported) }
