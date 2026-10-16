//! Counter-Strike 2 integration: Steam/CS2 discovery, config files, launch and
//! benchmark. Only official, user-level mechanisms are used (cfg files, video
//! file, Steam launch) — the game process is never touched.

pub mod benchmark;
pub mod config;
pub mod history;
pub mod launch;
pub mod presets;
pub mod vdf;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const APP_ID: &str = "730";
const STEAMID64_BASE: u64 = 76_561_197_960_265_728;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamUser {
    pub steam_id64: String,
    pub account_id: u32,
    pub persona_name: String,
    pub most_recent: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cs2Install {
    pub steam_path: Option<String>,
    pub game_path: Option<String>,
    pub installed: bool,
    pub running: bool,
    pub users: Vec<SteamUser>,
    pub launch_options: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoSettings {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub refresh_rate: Option<u32>,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cs2Config {
    #[serde(default)]
    pub convars: BTreeMap<String, String>,
    #[serde(default)]
    pub binds: BTreeMap<String, String>,
    #[serde(default)]
    pub video: VideoSettings,
    #[serde(default)]
    pub reload_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    /// "fps" | "competitive" | "visibility" | "lowend"
    pub tag: String,
    pub config: Cs2Config,
}

// ---------- Steam discovery ----------

/// Steam install root (e.g. C:\Program Files (x86)\Steam), used by tweaks for shader caches.
pub fn steam_path() -> Option<PathBuf> {
    steam_path_candidates().into_iter().find(|p| p.join("steam.exe").is_file() || p.join("steamapps").is_dir())
}

fn steam_path_candidates() -> Vec<PathBuf> {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        use crate::journal::{Hive, RegValue};
        use crate::win::registry;
        if let Ok(Some(RegValue::Sz(s))) = registry::read(Hive::Hkcu, r"Software\Valve\Steam", "SteamPath") {
            v.push(PathBuf::from(s.replace('/', "\\")));
        }
        if let Ok(Some(RegValue::Sz(s))) =
            registry::read(Hive::Hklm, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath")
        {
            v.push(PathBuf::from(s));
        }
        let pf86 = std::env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
        v.push(pf86.join("Steam"));
    }
    v
}

fn read_vdf(path: &Path) -> Option<vdf::Vdf> {
    let text = std::fs::read_to_string(path).ok()?;
    vdf::parse(&text).ok()
}

/// All Steam library roots (each contains `steamapps`).
pub fn library_folders(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    let doc = read_vdf(&steam.join("steamapps").join("libraryfolders.vdf"));
    if let Some(root) = doc.as_ref().and_then(|d| d.get("libraryfolders")) {
        for (_, entry) in root.entries() {
            // New format: { "path" "..." }, old format: "1" "D:\\Games".
            if let Some(p) = entry.get_str("path").or_else(|| entry.as_str()) {
                let p = PathBuf::from(p);
                if !libs.iter().any(|l| same_path(l, &p)) {
                    libs.push(p);
                }
            }
        }
    }
    libs
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_ascii_lowercase().trim_end_matches(['\\', '/'])
        == b.to_string_lossy().to_ascii_lowercase().trim_end_matches(['\\', '/'])
}

/// (library root, game root `<lib>/steamapps/common/<installdir>`) for app 730.
pub fn find_game() -> Option<(PathBuf, PathBuf)> {
    let steam = steam_path()?;
    for lib in library_folders(&steam) {
        let manifest = lib.join("steamapps").join(format!("appmanifest_{APP_ID}.acf"));
        let Some(doc) = read_vdf(&manifest) else { continue };
        let dir = doc
            .get("AppState")
            .and_then(|s| s.get_str("installdir"))
            .unwrap_or("Counter-Strike Global Offensive")
            .to_string();
        let game = lib.join("steamapps").join("common").join(dir);
        if game.is_dir() {
            return Some((lib, game));
        }
    }
    None
}

/// `<game>/game/csgo`
pub fn csgo_dir() -> Result<PathBuf> {
    find_game()
        .map(|(_, g)| g.join("game").join("csgo"))
        .ok_or_else(|| Error::Unavailable("instalação do CS2 não encontrada".into()))
}

/// Full path to game\bin\win64\cs2.exe, used by tweaks (FSO / GPU preference).
pub fn game_exe_path() -> Option<PathBuf> {
    let (_, game) = find_game()?;
    let exe = game.join("game").join("bin").join("win64").join("cs2.exe");
    exe.is_file().then_some(exe)
}

pub fn userdata_dir(account_id: u32) -> Result<PathBuf> {
    let steam = steam_path().ok_or_else(|| Error::Unavailable("Steam não encontrado".into()))?;
    Ok(steam.join("userdata").join(account_id.to_string()))
}

pub fn is_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case("cs2.exe"))
}

fn parse_users(doc: &vdf::Vdf) -> Vec<SteamUser> {
    let mut users = Vec::new();
    if let Some(root) = doc.get("users") {
        for (id, u) in root.entries() {
            let Ok(id64) = id.parse::<u64>() else { continue };
            if id64 < STEAMID64_BASE {
                continue;
            }
            let Ok(account_id) = u32::try_from(id64 - STEAMID64_BASE) else { continue };
            users.push(SteamUser {
                steam_id64: id.clone(),
                account_id,
                persona_name: u.get_str("PersonaName").unwrap_or_default().to_string(),
                most_recent: u.get_str("MostRecent") == Some("1"),
            });
        }
    }
    users.sort_by_key(|u| std::cmp::Reverse(u.most_recent));
    users
}

fn launch_options_from(doc: &vdf::Vdf) -> Option<String> {
    doc.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps", APP_ID, "LaunchOptions"])
        .and_then(vdf::Vdf::as_str)
        .map(str::to_string)
}

pub fn detect() -> Result<Cs2Install> {
    let steam = steam_path();
    let game = find_game();
    let users = steam
        .as_ref()
        .and_then(|s| read_vdf(&s.join("config").join("loginusers.vdf")))
        .map(|d| parse_users(&d))
        .unwrap_or_default();
    let launch_options = match (&steam, users.first()) {
        (Some(s), Some(u)) => read_vdf(
            &s.join("userdata").join(u.account_id.to_string()).join("config").join("localconfig.vdf"),
        )
        .and_then(|d| launch_options_from(&d)),
        _ => None,
    };
    Ok(Cs2Install {
        steam_path: steam.map(|p| p.to_string_lossy().into_owned()),
        game_path: game.as_ref().map(|(_, g)| g.to_string_lossy().into_owned()),
        installed: game.is_some(),
        running: is_running(),
        users,
        launch_options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_and_launch_options() {
        let users = vdf::parse(
            r#""users" {
                "76561198000000001" { "AccountName" "a" "PersonaName" "Alpha" "MostRecent" "0" }
                "76561198000000002" { "AccountName" "b" "PersonaName" "Bravo" "MostRecent" "1" }
            }"#,
        )
        .unwrap();
        let u = parse_users(&users);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].persona_name, "Bravo");
        assert_eq!(u[0].account_id, 39_734_274);
        assert!(u[0].most_recent);

        let lc = vdf::parse(
            r#""UserLocalConfigStore" { "Software" { "valve" { "Steam" { "apps" {
                "730" { "LaunchOptions" "-novid +exec vektor" } } } } } }"#,
        )
        .unwrap();
        assert_eq!(launch_options_from(&lc).as_deref(), Some("-novid +exec vektor"));
    }
}
