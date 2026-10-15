use crate::error::{Error, Result};
pub fn restore(_previous_guid: &str, _created_guid: Option<&str>) -> Result<()> { Err(Error::Unsupported) }
