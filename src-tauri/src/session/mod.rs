//! "Modo jogo": a boost applied while CS2 runs and undone when it exits.
//!
//! A watcher thread polls for `cs2.exe` every 2 s. When the game starts (and the
//! session is enabled) it closes chosen apps, pins CS2 to the recommended cores,
//! switches resolution and turns on vibrance while the game has focus. When the
//! game exits everything is reverted through the journal (tweak id
//! `game-session`) and the closed apps are reopened. A crash of Vektor during a
//! session is recovered at the next start ([`recover`]).

pub mod affinity;
mod apps;
mod focus;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::display::DisplayMode;
use crate::error::Result;
use crate::journal::{self, Undo};

pub(crate) const TWEAK_ID: &str = "game-session";
const CS2_EXE: &str = "cs2.exe";
const POLL: Duration = Duration::from_secs(2);
const MAX_CLOSE_APPS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AffinityMode {
    #[default]
    Off,
    Auto,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionConfig {
    pub enabled: bool,
    pub close_apps: Vec<String>,
    pub reopen_apps: bool,
    pub affinity: AffinityMode,
    pub affinity_mask: Option<String>,
    pub resolution: Option<DisplayMode>,
    pub vibrance: Option<i32>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            enabled: false,
            close_apps: Vec::new(),
            reopen_apps: true,
            affinity: AffinityMode::Off,
            affinity_mask: None,
            resolution: None,
            vibrance: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub active: bool,
    pub since: Option<String>,
    pub cs2_pid: Option<u32>,
    pub actions: Vec<String>,
    pub config: SessionConfig,
}

/// Normalise a config coming from disk or the UI (both untrusted).
pub(crate) fn sanitize(mut c: SessionConfig) -> SessionConfig {
    let mut apps: Vec<String> = Vec::new();
    for a in c.close_apps.iter().map(|a| a.trim().to_lowercase()) {
        if crate::tweaks::processes::is_allowed(&a) && !apps.contains(&a) && apps.len() < MAX_CLOSE_APPS {
            apps.push(a);
        }
    }
    c.close_apps = apps;

    let parsed = c.affinity_mask.as_deref().and_then(affinity::parse_mask);
    c.affinity_mask = parsed.map(affinity::fmt_mask);
    if c.affinity == AffinityMode::Custom && parsed.is_none() {
        c.affinity = AffinityMode::Off;
    }

    c.resolution = c.resolution.filter(|r| {
        (640..=7680).contains(&r.width) && (480..=4320).contains(&r.height) && (24..=500).contains(&r.refresh)
    });
    c.vibrance = c.vibrance.map(|v| v.clamp(0, 100));
    c
}

pub(crate) fn parse_config(bytes: &[u8]) -> SessionConfig {
    serde_json::from_slice::<SessionConfig>(bytes).map(sanitize).unwrap_or_default()
}

fn config_path() -> PathBuf {
    journal::data_dir().join("session.json")
}

fn state_path() -> PathBuf {
    journal::data_dir().join("session-state.json")
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(journal::data_dir())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub(crate) fn load_config() -> SessionConfig {
    std::fs::read(config_path()).map(|b| parse_config(&b)).unwrap_or_default()
}

/// Apps closed by the current session, persisted so a crash can still reopen them.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PersistedState {
    reopen: Vec<PathBuf>,
}

fn load_state() -> PersistedState {
    std::fs::read(state_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_state(s: &PersistedState) -> Result<()> {
    write_atomic(&state_path(), &serde_json::to_vec_pretty(s)?)
}

// ---------- runtime ----------

#[derive(Default)]
struct Runtime {
    active: bool,
    since: Option<String>,
    pid: Option<u32>,
    actions: Vec<String>,
    focus: Option<focus::FocusLoop>,
    /// PID the user stopped manually; not auto-boosted again.
    suppressed_pid: Option<u32>,
}

static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

fn rt() -> MutexGuard<'static, Option<Runtime>> {
    // Never let a poisoned lock kill the watcher.
    RUNTIME.lock().unwrap_or_else(|e| e.into_inner())
}

fn with_rt<T>(f: impl FnOnce(&mut Runtime) -> T) -> T {
    let mut g = rt();
    f(g.get_or_insert_with(Runtime::default))
}

fn snapshot(config: SessionConfig) -> SessionStatus {
    with_rt(|r| SessionStatus {
        active: r.active,
        since: r.since.clone(),
        cs2_pid: r.pid,
        actions: r.actions.clone(),
        config,
    })
}

pub fn status() -> Result<SessionStatus> {
    Ok(snapshot(load_config()))
}

pub fn set_config(c: SessionConfig) -> Result<SessionStatus> {
    let c = sanitize(c);
    write_atomic(&config_path(), &serde_json::to_vec_pretty(&c)?)?;
    Ok(snapshot(c))
}

pub fn stop() -> Result<SessionStatus> {
    let pid = with_rt(|r| if r.active { r.pid } else { None });
    if pid.is_some() || journal::is_applied(TWEAK_ID) {
        end();
        with_rt(|r| r.suppressed_pid = pid);
    }
    status()
}

fn find_cs2(sys: &mut System) -> Option<u32> {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .iter()
        .find(|(_, p)| p.name().to_string_lossy().eq_ignore_ascii_case(CS2_EXE))
        .map(|(pid, _)| pid.as_u32())
}

/// Primary display device and its current mode, read through the display
/// module's serialized DTO (keeps this module independent of its field layout).
fn primary_display() -> Option<(String, DisplayMode)> {
    let list = serde_json::to_value(crate::display::modes::list().ok()?).ok()?;
    let arr = list.as_array()?;
    let d = arr.iter().find(|d| d["primary"].as_bool() == Some(true)).or_else(|| arr.first())?;
    let device = d["device"].as_str()?.to_string();
    let current: DisplayMode = serde_json::from_value(d["current"].clone()).ok()?;
    Some((device, current))
}

fn begin(pid: u32) {
    let cfg = load_config();
    let mut actions: Vec<String> = Vec::new();

    // 1) Close apps (remember executables for reopening).
    if !cfg.close_apps.is_empty() {
        let closed = apps::close(&cfg.close_apps);
        let mut state = load_state();
        for c in &closed {
            actions.push(format!("Fechou {}", c.label));
            if let Some(p) = c.exe.as_ref().filter(|p| !state.reopen.contains(p)) {
                state.reopen.push(p.clone());
            }
        }
        if let Err(e) = save_state(&state) {
            actions.push(format!("Erro ao guardar apps para reabrir: {e}"));
        }
    }

    // 2) CPU affinity (dies with the process — nothing to journal).
    match affinity::resolve(cfg.affinity, cfg.affinity_mask.as_deref()) {
        Ok(Some((mask, label))) => match affinity::apply(pid, mask) {
            Ok(()) => actions.push(format!("Afinidade: {label}")),
            Err(e) => actions.push(format!("Erro na afinidade: {e}")),
        },
        Ok(None) => {}
        Err(e) => actions.push(format!("Erro na afinidade: {e}")),
    }

    // 3) Resolution: journal the current mode before switching.
    if let Some(r) = cfg.resolution {
        match primary_display() {
            Some((device, cur)) if cur != r => {
                let undo = Undo::DisplayMode { device: device.clone(), width: cur.width, height: cur.height, refresh: cur.refresh };
                match journal::record(TWEAK_ID, undo) {
                    Ok(()) => match crate::display::modes::set_mode(&device, r.width, r.height, r.refresh) {
                        Ok(_) => actions.push(format!("Resolução {}×{}@{}", r.width, r.height, r.refresh)),
                        Err(e) => actions.push(format!("Erro na resolução: {e}")),
                    },
                    Err(e) => actions.push(format!("Erro no registo da resolução: {e}")),
                }
            }
            Some(_) => {}
            None => actions.push("Erro na resolução: ecrã principal não encontrado".into()),
        }
    }

    // 4) Vibrance while CS2 has focus.
    let mut focus = None;
    if let Some(level) = cfg.vibrance {
        match crate::display::vibrance::get_level("") {
            Ok(original) => match journal::record(TWEAK_ID, Undo::Vibrance { device: String::new(), previous: original }) {
                Ok(()) => {
                    focus = Some(focus::FocusLoop::start(pid, level, original));
                    actions.push(format!("Vibrance {level}% com o CS2 em foco"));
                }
                Err(e) => actions.push(format!("Erro no registo da vibrance: {e}")),
            },
            Err(e) => actions.push(format!("Erro na vibrance: {e}")),
        }
    }

    if actions.is_empty() {
        actions.push("Sessão ativa (sem ações configuradas)".into());
    }
    let since = crate::memory::now_iso8601();
    with_rt(|r| {
        r.active = true;
        r.since = Some(since);
        r.pid = Some(pid);
        r.actions = actions;
        r.focus = focus;
    });
}

fn end() {
    // Stop the vibrance loop first so it cannot re-apply after the revert.
    if let Some(f) = with_rt(|r| r.focus.take()) {
        f.stop();
    }
    let mut actions = with_rt(|r| std::mem::take(&mut r.actions));
    if journal::is_applied(TWEAK_ID) {
        match journal::revert(TWEAK_ID) {
            Ok(()) => actions.push("Definições repostas".into()),
            Err(e) => actions.push(format!("Erro ao repor: {e}")),
        }
    }
    let state = load_state();
    if !state.reopen.is_empty() && load_config().reopen_apps {
        for label in apps::reopen(&state.reopen) {
            actions.push(format!("Reabriu {label}"));
        }
    }
    let _ = std::fs::remove_file(state_path());
    actions.push("Sessão terminada".into());
    with_rt(|r| {
        r.active = false;
        r.since = None;
        r.pid = None;
        r.actions = actions;
    });
}

/// Called once at startup: undo a session left behind by a crash.
pub fn recover() {
    let leftover = journal::is_applied(TWEAK_ID) || !load_state().reopen.is_empty();
    if leftover && find_cs2(&mut System::new()).is_none() {
        end();
    }
}

/// Start the background watcher thread (called once from lib.rs setup).
pub fn spawn_watcher() {
    let _ = std::thread::Builder::new().name("vektor-session".into()).spawn(|| {
        let mut sys = System::new();
        loop {
            tick(&mut sys);
            std::thread::sleep(POLL);
        }
    });
}

fn tick(sys: &mut System) {
    let pid = find_cs2(sys);
    let (active, active_pid, suppressed) = with_rt(|r| (r.active, r.pid, r.suppressed_pid));
    match pid {
        Some(p) if !active => {
            if suppressed != Some(p) && load_config().enabled {
                begin(p);
            }
        }
        Some(p) if active_pid != Some(p) => {
            // Game restarted between polls: close the old session, start fresh.
            end();
            if load_config().enabled {
                begin(p);
            }
        }
        Some(_) => {}
        None => {
            if active {
                end();
            }
            with_rt(|r| r.suppressed_pid = None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_on_garbage() {
        assert_eq!(parse_config(b"not json"), SessionConfig::default());
        assert!(SessionConfig::default().reopen_apps);
        assert_eq!(parse_config(b"{}"), SessionConfig::default());
    }

    #[test]
    fn sanitizes_untrusted_values() {
        let raw = br#"{
            "enabled": true,
            "closeApps": ["Discord.exe", "cs2.exe", "steam.exe", "discord.exe", "explorer.exe"],
            "affinity": "custom",
            "affinityMask": "ff",
            "resolution": {"width": 1280, "height": 960, "refresh": 240},
            "vibrance": 250
        }"#;
        let c = parse_config(raw);
        assert!(c.enabled);
        assert_eq!(c.close_apps, vec!["discord.exe".to_string()]);
        assert_eq!(c.affinity, AffinityMode::Custom);
        assert_eq!(c.affinity_mask.as_deref(), Some("0xFF"));
        assert_eq!(c.resolution, Some(DisplayMode { width: 1280, height: 960, refresh: 240 }));
        assert_eq!(c.vibrance, Some(100));
    }

    #[test]
    fn invalid_custom_mask_disables_affinity() {
        let c = parse_config(br#"{"affinity":"custom","affinityMask":"0x0"}"#);
        assert_eq!(c.affinity, AffinityMode::Off);
        assert_eq!(c.affinity_mask, None);
    }

    #[test]
    fn rejects_absurd_resolution() {
        let c = parse_config(br#"{"resolution":{"width":10,"height":10,"refresh":1000}}"#);
        assert_eq!(c.resolution, None);
    }

    #[test]
    fn unknown_affinity_value_falls_back_to_defaults() {
        assert_eq!(parse_config(br#"{"affinity":"turbo"}"#), SessionConfig::default());
    }
}
