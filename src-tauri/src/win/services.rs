use crate::error::{Error, Result};
pub fn restore(_name: &str, _start: u32, _running: bool) -> Result<()> { Err(Error::Unsupported) }
