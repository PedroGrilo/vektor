use crate::error::{Error, Result};
use super::Cs2Config;
pub fn read(_account_id: u32) -> Result<Cs2Config> { Err(Error::Unsupported) }
pub fn apply(_account_id: u32, _c: &Cs2Config) -> Result<()> { Err(Error::Unsupported) }
