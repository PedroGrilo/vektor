//! Finding installed games from the launchers' own metadata. Read-only.

use std::path::{Path, PathBuf};

use super::Source;

/// A game found on disk.
#[derive(Debug, Clone)]
pub struct Detected {
    pub id: String,
    pub name: String,
    pub source: Source,
    pub install_dir: PathBuf,
    /// Executable named by the launcher itself, when it says.
    pub preferred_exe: Option<PathBuf>,
}

pub fn all() -> Vec<Detected> {
    let mut v = Vec::new();
    v.extend(steam());
    v.extend(epic());
    v.extend(gog());
    v.extend(ubisoft());
    v.extend(riot());
    v.sort_by_key(|g| g.name.to_lowercase());
    v
}

// ---------- Steam ----------

/// Steam entries that are runtimes or tools, not games.
pub fn steam_skip(app_id: &str, name: &str) -> bool {
    const IDS: &[&str] = &["228980"]; // Steamworks Common Redistributables
    const PREFIXES: &[&str] = &["proton", "steam linux runtime", "steamworks common", "steamvr"];
    let n = name.to_lowercase();
    IDS.contains(&app_id)
        || PREFIXES.iter().any(|p| n.starts_with(p))
        || n.contains("dedicated server")
        || n.ends_with(" sdk")
}

fn steam() -> Vec<Detected> {
    let Some(steam) = crate::cs2::steam_path() else { return Vec::new() };
    let mut out = Vec::new();
    for lib in crate::cs2::library_folders(&steam) {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let file = e.file_name().to_string_lossy().to_string();
            if !(file.starts_with("appmanifest_") && file.ends_with(".acf")) {
                continue;
            }
            let Some(doc) = crate::cs2::read_vdf(&e.path()) else { continue };
            let Some(st) = doc.get("AppState") else { continue };
            let (Some(app_id), Some(name), Some(dir)) = (st.get_str("appid"), st.get_str("name"), st.get_str("installdir"))
            else {
                continue;
            };
            if steam_skip(app_id, name) {
                continue;
            }
            let install_dir = apps.join("common").join(dir);
            if !install_dir.is_dir() || out.iter().any(|d: &Detected| d.id == format!("steam:{app_id}")) {
                continue;
            }
            let preferred_exe = if app_id == crate::cs2::APP_ID { crate::cs2::game_exe_path() } else { None };
            out.push(Detected {
                id: format!("steam:{app_id}"),
                name: name.to_string(),
                source: Source::Steam,
                install_dir,
                preferred_exe,
            });
        }
    }
    out
}

// ---------- Epic Games ----------

fn program_data() -> PathBuf {
    std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

/// One `.item` manifest from `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests`.
pub fn epic_item(json: &str) -> Option<Detected> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let cats = v["AppCategories"].as_array();
    if cats.is_some_and(|c| !c.iter().any(|x| x.as_str() == Some("games"))) {
        return None;
    }
    if v["bIsIncompleteInstall"].as_bool() == Some(true) {
        return None;
    }
    let name = v["DisplayName"].as_str()?.to_string();
    let app = v["AppName"].as_str().or_else(|| v["MainGameAppName"].as_str())?;
    let dir = PathBuf::from(v["InstallLocation"].as_str()?);
    let preferred_exe = v["LaunchExecutable"].as_str().filter(|s| !s.is_empty()).map(|s| dir.join(s));
    Some(Detected { id: format!("epic:{app}"), name, source: Source::Epic, install_dir: dir, preferred_exe })
}

fn epic() -> Vec<Detected> {
    let dir = program_data().join(r"Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("item")))
        .filter_map(|e| epic_item(&std::fs::read_to_string(e.path()).ok()?))
        .filter(|d| d.install_dir.is_dir())
        .collect()
}

// ---------- GOG / Ubisoft (registry) ----------

#[cfg(windows)]
fn reg_str(key: &str, name: &str) -> Option<String> {
    use crate::journal::{Hive, RegValue};
    match crate::win::registry::read(Hive::Hklm, key, name) {
        Ok(Some(RegValue::Sz(s))) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

#[cfg(windows)]
fn gog() -> Vec<Detected> {
    use crate::journal::Hive;
    const ROOT: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games";
    crate::win::registry::subkeys(Hive::Hklm, ROOT)
        .into_iter()
        .filter_map(|id| {
            let key = format!(r"{ROOT}\{id}");
            let dir = PathBuf::from(reg_str(&key, "path")?);
            let name = reg_str(&key, "gameName").unwrap_or_else(|| id.clone());
            let preferred_exe = reg_str(&key, "exe").map(PathBuf::from);
            dir.is_dir().then(|| Detected { id: format!("gog:{id}"), name, source: Source::Gog, install_dir: dir, preferred_exe })
        })
        .collect()
}

#[cfg(windows)]
fn ubisoft() -> Vec<Detected> {
    use crate::journal::Hive;
    const ROOT: &str = r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs";
    crate::win::registry::subkeys(Hive::Hklm, ROOT)
        .into_iter()
        .filter_map(|id| {
            let dir = PathBuf::from(reg_str(&format!(r"{ROOT}\{id}"), "InstallDir")?.replace('/', "\\"));
            let name = dir.file_name()?.to_string_lossy().to_string();
            dir.is_dir().then(|| Detected { id: format!("ubisoft:{id}"), name, source: Source::Ubisoft, install_dir: dir, preferred_exe: None })
        })
        .collect()
}

#[cfg(not(windows))]
fn gog() -> Vec<Detected> {
    Vec::new()
}

#[cfg(not(windows))]
fn ubisoft() -> Vec<Detected> {
    Vec::new()
}

// ---------- Riot ----------

/// Riot products Vektor knows: (product id, name, game exe relative to the install path).
const RIOT: &[(&str, &str, &str)] = &[
    ("valorant", "VALORANT", r"ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe"),
    ("league_of_legends", "League of Legends", r"Game\League of Legends.exe"),
    ("bacon", "Legends of Runeterra", "LoR.exe"),
];

/// `product_install_full_path: "C:/Riot Games/VALORANT/live"` from a product_settings.yaml.
pub fn riot_install_path(yaml: &str) -> Option<PathBuf> {
    yaml.lines().find_map(|l| {
        let v = l.trim().strip_prefix("product_install_full_path:")?.trim().trim_matches(['"', '\'']);
        (!v.is_empty()).then(|| PathBuf::from(v.replace('/', std::path::MAIN_SEPARATOR_STR)))
    })
}

fn riot() -> Vec<Detected> {
    let meta = program_data().join(r"Riot Games\Metadata");
    RIOT.iter()
        .filter_map(|(id, name, exe)| {
            let file = meta.join(format!("{id}.live")).join(format!("{id}.live.product_settings.yaml"));
            let dir = riot_install_path(&std::fs::read_to_string(file).ok()?)?;
            let exe = dir.join(exe);
            dir.is_dir().then(|| Detected {
                id: format!("riot:{id}"),
                name: name.to_string(),
                source: Source::Riot,
                preferred_exe: exe.is_file().then_some(exe),
                install_dir: dir,
            })
        })
        .collect()
}

/// True when `p` is inside `dir` (case-insensitive, for Windows paths).
pub fn is_inside(p: &Path, dir: &Path) -> bool {
    let a = p.to_string_lossy().to_lowercase().replace('/', "\\");
    let b = dir.to_string_lossy().to_lowercase().replace('/', "\\");
    let b = b.trim_end_matches('\\');
    a.starts_with(b) && a[b.len()..].starts_with('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_steam_runtimes() {
        assert!(steam_skip("228980", "Steamworks Common Redistributables"));
        assert!(steam_skip("1493710", "Proton Experimental"));
        assert!(steam_skip("1", "Counter-Strike 2 Dedicated Server"));
        assert!(!steam_skip("730", "Counter-Strike 2"));
    }

    #[test]
    fn parses_epic_item() {
        let j = r#"{"DisplayName":"Fortnite","AppName":"Fortnite","InstallLocation":"C:\\Games\\Fortnite",
            "LaunchExecutable":"FortniteGame/Binaries/Win64/FortniteClient-Win64-Shipping.exe","AppCategories":["public","games","applications"]}"#;
        let d = epic_item(j).unwrap();
        assert_eq!(d.id, "epic:Fortnite");
        assert!(d.preferred_exe.unwrap().to_string_lossy().ends_with("FortniteClient-Win64-Shipping.exe"));
    }

    #[test]
    fn epic_non_games_are_skipped() {
        let j = r#"{"DisplayName":"Unreal Engine","AppName":"UE_5","InstallLocation":"C:\\UE","AppCategories":["engines"]}"#;
        assert!(epic_item(j).is_none());
    }

    #[test]
    fn reads_riot_install_path() {
        let y = "product_install_full_path: \"C:/Riot Games/VALORANT/live\"\nproduct_install_root: \"C:/Riot Games\"\n";
        let p = riot_install_path(y).unwrap();
        assert!(p.to_string_lossy().contains("VALORANT"));
    }

    #[test]
    fn inside_is_case_insensitive_and_strict() {
        assert!(is_inside(Path::new(r"D:\Games\X\bin\x.exe"), Path::new(r"d:\games\x")));
        assert!(!is_inside(Path::new(r"D:\Games\XY\x.exe"), Path::new(r"D:\Games\X")));
    }
}
