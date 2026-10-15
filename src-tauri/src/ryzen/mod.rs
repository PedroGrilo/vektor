//! STUB — owned by worker "gpu/ryzen".
use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct RyzenStatus {}
pub fn status() -> Result<RyzenStatus> { Err(Error::Unsupported) }
pub fn apply_curve(_o: i32) -> Result<RyzenStatus> { Err(Error::Unsupported) }
pub fn clear_curve() -> Result<RyzenStatus> { Err(Error::Unsupported) }
pub fn set_all_core(_o: i32) -> Result<()> { Err(Error::Unsupported) }
