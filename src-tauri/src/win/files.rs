use std::path::Path;
use crate::error::{Error, Result};
pub fn restore(_path: &Path, _backup: Option<&Path>) -> Result<()> { Err(Error::Unsupported) }
pub fn set_readonly(_path: &Path, _ro: bool) -> Result<()> { Err(Error::Unsupported) }
