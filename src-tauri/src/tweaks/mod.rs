//! STUB — owned by worker "tweaks/system".
pub mod cache;
pub mod processes;
use serde::Serialize;
use crate::error::{Error, Result};
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct TweakInfo {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct CleanReport {}
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct ProcInfo {}
pub fn list() -> Vec<TweakInfo> { vec![] }
pub fn apply(_id: &str) -> Result<TweakInfo> { Err(Error::Unsupported) }
pub fn revert(_id: &str) -> Result<TweakInfo> { Err(Error::Unsupported) }
