use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct BenchStatus {}
pub fn start() -> Result<()> { Err(Error::Unsupported) }
pub fn poll() -> Result<BenchStatus> { Err(Error::Unsupported) }
