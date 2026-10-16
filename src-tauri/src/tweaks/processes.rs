//! Closing non-essential background apps before playing. Only executables in
//! the fixed allowlist below can ever be listed or terminated.

use std::collections::BTreeMap;

use sysinfo::{ProcessesToUpdate, System};

use super::ProcInfo;
use crate::error::{Error, Result};

/// (lowercase executable name, friendly label). Steam, CS2, anti-cheats,
/// drivers and Windows processes must never be added here.
const BACKGROUND_APPS: &[(&str, &str)] = &[
    ("discord.exe", "Discord"),
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("firefox.exe", "Firefox"),
    ("opera.exe", "Opera"),
    ("brave.exe", "Brave"),
    ("spotify.exe", "Spotify"),
    ("obs64.exe", "OBS Studio"),
    ("ms-teams.exe", "Microsoft Teams"),
    ("teams.exe", "Microsoft Teams (clássico)"),
    ("onedrive.exe", "OneDrive"),
    ("dropbox.exe", "Dropbox"),
    ("epicgameslauncher.exe", "Epic Games Launcher"),
    ("battle.net.exe", "Battle.net"),
    ("eadesktop.exe", "EA app"),
    ("ubisoftconnect.exe", "Ubisoft Connect"),
    ("galaxyclient.exe", "GOG Galaxy"),
    ("whatsapp.exe", "WhatsApp"),
    ("telegram.exe", "Telegram"),
    ("slack.exe", "Slack"),
    ("skype.exe", "Skype"),
    ("zoom.exe", "Zoom"),
    ("adobe desktop service.exe", "Adobe Creative Cloud"),
    ("creative cloud.exe", "Adobe Creative Cloud"),
    ("icue.exe", "Corsair iCUE"),
    ("razer synapse 3.exe", "Razer Synapse"),
];

fn label_of(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    BACKGROUND_APPS.iter().find(|(exe, _)| *exe == lower).map(|(_, l)| *l)
}

/// True when `name` (executable file name, any case) is in the background-apps allowlist.
pub fn is_allowed(name: &str) -> bool {
    label_of(name).is_some()
}

/// Friendly label of an allowlisted executable.
pub fn label(name: &str) -> Option<&'static str> {
    label_of(name)
}

fn processes() -> System {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys
}

/// One entry per allowlisted executable currently running (memory summed
/// over all its processes; `pid` is the first one found).
pub fn list_background_apps() -> Result<Vec<ProcInfo>> {
    let sys = processes();
    let mut by_name: BTreeMap<String, ProcInfo> = BTreeMap::new();
    for (pid, p) in sys.processes() {
        let name = p.name().to_string_lossy().into_owned();
        let Some(label) = label_of(&name) else { continue };
        let mem_mb = p.memory() / (1024 * 1024);
        by_name
            .entry(name.to_lowercase())
            .and_modify(|e| e.memory_mb += mem_mb)
            .or_insert(ProcInfo { name, pid: pid.as_u32(), memory_mb: mem_mb, label: label.into() });
    }
    let mut list: Vec<ProcInfo> = by_name.into_values().collect();
    list.sort_by_key(|p| std::cmp::Reverse(p.memory_mb));
    Ok(list)
}

/// Terminate every process whose executable name is in `names` AND in the
/// allowlist. Returns how many processes were terminated.
pub fn kill_by_names(names: &[String]) -> Result<u32> {
    let wanted: Vec<String> = names
        .iter()
        .map(|n| n.to_lowercase())
        .filter(|n| label_of(n).is_some())
        .collect();
    if wanted.is_empty() {
        return Err(Error::InvalidInput("nenhuma aplicação permitida na lista".into()));
    }
    let sys = processes();
    let killed = sys
        .processes()
        .values()
        .filter(|p| wanted.contains(&p.name().to_string_lossy().to_lowercase()))
        .filter(|p| p.kill())
        .count();
    Ok(killed as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_excludes_critical_processes() {
        for critical in ["steam.exe", "steamwebhelper.exe", "cs2.exe", "explorer.exe", "csrss.exe"] {
            assert!(label_of(critical).is_none(), "{critical} must not be killable");
        }
        assert_eq!(label_of("Discord.exe"), Some("Discord"));
    }
}
