use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct CpuTopology {}
pub fn topology() -> Result<CpuTopology> { Err(Error::Unsupported) }
