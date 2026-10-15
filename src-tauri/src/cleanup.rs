//! STUB — owned by worker "cleanup".
use serde::Serialize;
use crate::error::{Error, Result};
use crate::tweaks::CleanReport;
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct CleanTarget {}
pub fn scan() -> Result<Vec<CleanTarget>> { Err(Error::Unsupported) }
pub fn run(_ids: &[String]) -> Result<CleanReport> { Err(Error::Unsupported) }
