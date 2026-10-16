//! STUB — owned by worker "display".
pub mod modes;
pub mod vibrance;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)] #[serde(rename_all = "camelCase")]
pub struct DisplayMode { pub width: u32, pub height: u32, pub refresh: u32 }
#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct DisplayInfo {}
