use crate::error::{Error, Result};
use super::ProcInfo;
pub fn list_background_apps() -> Result<Vec<ProcInfo>> { Err(Error::Unsupported) }
pub fn kill_by_names(_names: &[String]) -> Result<u32> { Err(Error::Unsupported) }
