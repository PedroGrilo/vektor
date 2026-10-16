//! Display modes (resolution/refresh for the game session) and digital vibrance.

pub mod modes;
pub mod vibrance;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    /// e.g. `\\.\DISPLAY1`
    pub device: String,
    pub name: String,
    pub primary: bool,
    pub current: DisplayMode,
    pub modes: Vec<DisplayMode>,
}
