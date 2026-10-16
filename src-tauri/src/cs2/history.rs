use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct BenchRecord {}
pub fn list() -> Result<Vec<BenchRecord>> { Err(Error::Unsupported) }
pub fn delete(_id: &str) -> Result<Vec<BenchRecord>> { Err(Error::Unsupported) }
pub fn set_note(_id: &str, _note: &str) -> Result<Vec<BenchRecord>> { Err(Error::Unsupported) }
