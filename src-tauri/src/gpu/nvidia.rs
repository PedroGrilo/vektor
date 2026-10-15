use serde::Deserialize;
use crate::error::{Error, Result};
#[derive(Deserialize)] #[serde(rename_all = "camelCase")] pub struct NvPreset {}
pub fn apply_cs2_profile(_p: &NvPreset) -> Result<()> { Err(Error::Unsupported) }
pub fn restore_setting(_profile: &str, _id: u32, _prev: Option<u32>) -> Result<()> { Err(Error::Unsupported) }
pub fn delete_profile(_profile: &str) -> Result<()> { Err(Error::Unsupported) }
