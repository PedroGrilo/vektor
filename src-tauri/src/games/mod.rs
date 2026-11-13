//! Game library: every installed game Vektor can optimise, not just CS2.
//!
//! Games come from the launchers (Steam, Epic, GOG, Ubisoft, Riot) plus ones
//! the user adds by hand. Per game, Vektor can apply the same three per-exe
//! tweaks it has for CS2 (High CPU priority, high-performance GPU, fullscreen
//! optimisations off) and add it to the Game Mode watcher. User choices live in
//! `%ProgramData%\Vektor\games.json`; the tweaks themselves go through the journal.

pub mod detect;
pub mod exe;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::error::{Error, Result};
use crate::journal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Steam,
    Epic,
    Gog,
    Ubisoft,
    Riot,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakKind {
    Priority,
    Gpu,
    Fso,
}

const KINDS: [TweakKind; 3] = [TweakKind::Priority, TweakKind::Gpu, TweakKind::Fso];

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameTweaks {
    pub priority: bool,
    pub gpu: bool,
    pub fso: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    pub source: Source,
    pub install_dir: Option<String>,
    /// Chosen executable (full path), if any was found.
    pub exe: Option<String>,
    /// Other likely executables, best first.
    pub candidates: Vec<String>,
    pub tweaks: GameTweaks,
    /// Watched by Game Mode.
    pub in_session: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningProcess {
    pub name: String,
    pub exe: String,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManualGame {
    id: String,
    name: String,
    exe: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Library {
    manual: Vec<ManualGame>,
    /// game id -> executable chosen by the user
    exe: BTreeMap<String, String>,
    /// detected games the user removed from the list
    hidden: Vec<String>,
}

fn library_path() -> PathBuf {
    journal::data_dir().join("games.json")
}

fn load_library() -> Library {
    std::fs::read(library_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_library(l: &Library) -> Result<()> {
    std::fs::create_dir_all(journal::data_dir())?;
    let tmp = library_path().with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(l)?)?;
    std::fs::rename(tmp, library_path())?;
    Ok(())
}

/// Detected games with their exe candidates (folder scans are slow; cached).
struct Scanned {
    game: detect::Detected,
    candidates: Vec<PathBuf>,
}

static CACHE: Mutex<Option<Vec<Scanned>>> = Mutex::new(None);

fn scan() -> Vec<Scanned> {
    detect::all()
        .into_iter()
        .map(|g| {
            let folder = g.install_dir.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            let mut candidates = exe::candidates(&g.install_dir, &[&g.name, &folder]);
            if let Some(p) = g.preferred_exe.as_ref().filter(|p| p.is_file()) {
                candidates.retain(|c| !same_file(c, p));
                candidates.insert(0, p.clone());
                candidates.truncate(exe::MAX_CANDIDATES);
            }
            Scanned { game: g, candidates }
        })
        .collect()
}

fn same_file(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

/// File name of a Windows path, lowercase (also splits on `\\` when built elsewhere).
pub fn file_name_lower(exe: &str) -> String {
    exe.rsplit(['\\', '/']).next().unwrap_or_default().to_lowercase()
}

fn is_cs2(exe: &str) -> bool {
    file_name_lower(exe) == "cs2.exe"
}

/// Journal id of one per-game tweak. CS2 keeps the ids of its catalog tweaks
/// so both pages show the same state.
pub fn tweak_id(game_id: &str, exe: &str, kind: TweakKind) -> String {
    match (is_cs2(exe), kind) {
        (true, TweakKind::Priority) => "cs2-high-priority".into(),
        (true, TweakKind::Gpu) => "cs2-gpu-high-performance".into(),
        (true, TweakKind::Fso) => "cs2-fso-off".into(),
        (false, TweakKind::Priority) => format!("game:{game_id}:priority"),
        (false, TweakKind::Gpu) => format!("game:{game_id}:gpu"),
        (false, TweakKind::Fso) => format!("game:{game_id}:fso"),
    }
}

fn running_names() -> Vec<String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect()
}

/// Every game, detected and manual. `refresh` rescans the launchers and folders.
pub fn list(refresh: bool) -> Result<Vec<GameInfo>> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if refresh || cache.is_none() {
        *cache = Some(scan());
    }
    let lib = load_library();
    let session = crate::session::load_config();
    let running = running_names();
    let applied = journal::applied_ids();

    let build = |id: String, name: String, source: Source, dir: Option<&Path>, exe: Option<String>, candidates: Vec<String>| {
        let ex = exe.clone().unwrap_or_default();
        let is = |k| exe.is_some() && applied.contains(&tweak_id(&id, &ex, k));
        let file = file_name_lower(&ex);
        GameInfo {
            tweaks: GameTweaks { priority: is(TweakKind::Priority), gpu: is(TweakKind::Gpu), fso: is(TweakKind::Fso) },
            in_session: exe.is_some() && session.games.iter().any(|g| g.exe == file),
            running: exe.is_some() && running.contains(&file),
            install_dir: dir.map(|d| d.to_string_lossy().to_string()),
            id,
            name,
            source,
            exe,
            candidates,
        }
    };

    let mut out = Vec::new();
    for s in cache.as_ref().into_iter().flatten() {
        if lib.hidden.contains(&s.game.id) {
            continue;
        }
        let candidates: Vec<String> = s.candidates.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let exe = lib.exe.get(&s.game.id).cloned().or_else(|| candidates.first().cloned());
        out.push(build(s.game.id.clone(), s.game.name.clone(), s.game.source, Some(&s.game.install_dir), exe, candidates));
    }
    for m in &lib.manual {
        let dir = Path::new(&m.exe).parent().map(Path::to_path_buf);
        out.push(build(m.id.clone(), m.name.clone(), Source::Manual, dir.as_deref(), Some(m.exe.clone()), vec![m.exe.clone()]));
    }
    Ok(out)
}

fn get(id: &str) -> Result<GameInfo> {
    list(false)?
        .into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| Error::InvalidInput(format!("jogo desconhecido: {id}")))
}

/// An existing `.exe` outside the Windows folder.
fn validate_exe(exe: &str) -> Result<PathBuf> {
    let p = PathBuf::from(exe.trim().trim_matches('"'));
    let ok_ext = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    if !p.is_absolute() || !ok_ext {
        return Err(Error::InvalidInput("indica o caminho completo de um ficheiro .exe".into()));
    }
    if !p.is_file() {
        return Err(Error::InvalidInput(format!("ficheiro não encontrado: {}", p.display())));
    }
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    if detect::is_inside(&p, &windir) {
        return Err(Error::InvalidInput("programas do Windows não podem ser adicionados como jogo".into()));
    }
    Ok(p)
}

fn any_tweak_applied(g: &GameInfo) -> bool {
    g.tweaks.priority || g.tweaks.gpu || g.tweaks.fso
}

pub fn set_exe(id: &str, exe: &str) -> Result<GameInfo> {
    let g = get(id)?;
    if any_tweak_applied(&g) {
        return Err(Error::InvalidInput("reverte as otimizações deste jogo antes de trocar o executável".into()));
    }
    let p = validate_exe(exe)?;
    let mut lib = load_library();
    let p = p.to_string_lossy().to_string();
    if let Some(m) = lib.manual.iter_mut().find(|m| m.id == id) {
        m.exe = p;
    } else {
        lib.exe.insert(id.to_string(), p);
    }
    save_library(&lib)?;
    get(id)
}

pub fn add_manual(exe: &str, name: Option<String>) -> Result<GameInfo> {
    let p = validate_exe(exe)?;
    let path = p.to_string_lossy().to_string();
    let mut lib = load_library();
    if list(false)?.iter().any(|g| g.exe.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&path))) {
        return Err(Error::InvalidInput("este jogo já está na lista".into()));
    }
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Jogo".into());
    let name = name.map(|n| n.trim().chars().take(80).collect::<String>()).filter(|n| !n.is_empty()).unwrap_or(stem);
    let id = format!("manual:{}", exe::norm(&path));
    lib.manual.push(ManualGame { id: id.clone(), name, exe: path });
    save_library(&lib)?;
    get(&id)
}

/// Remove a game from the list (manual) or hide it (detected). Its tweaks must be reverted first.
pub fn remove(id: &str) -> Result<()> {
    let g = get(id)?;
    if any_tweak_applied(&g) {
        return Err(Error::InvalidInput("reverte as otimizações deste jogo antes de o remover".into()));
    }
    if g.in_session {
        set_session(id, false)?;
    }
    let mut lib = load_library();
    lib.manual.retain(|m| m.id != id);
    lib.exe.remove(id);
    if g.source != Source::Manual && !lib.hidden.iter().any(|h| h == id) {
        lib.hidden.push(id.to_string());
    }
    save_library(&lib)
}

/// Bring back every detected game the user hid.
pub fn unhide_all() -> Result<()> {
    let mut lib = load_library();
    lib.hidden.clear();
    save_library(&lib)
}

pub fn set_tweak(id: &str, kind: TweakKind, on: bool) -> Result<GameInfo> {
    let g = get(id)?;
    let exe = g.exe.clone().ok_or_else(|| Error::Unavailable("executável do jogo não encontrado".into()))?;
    let tid = tweak_id(id, &exe, kind);
    if !on {
        journal::revert(&tid)?;
        return get(id);
    }
    if journal::is_applied(&tid) {
        return get(id);
    }
    if !cfg!(windows) {
        return Err(Error::Unsupported);
    }
    let r = match kind {
        TweakKind::Priority => crate::tweaks::set_exe_high_priority(&tid, &file_name_lower(&exe)),
        TweakKind::Gpu => crate::tweaks::set_exe_gpu_high_performance(&tid, &exe),
        TweakKind::Fso => crate::tweaks::set_exe_fso_off(&tid, &exe),
    };
    if let Err(e) = r {
        let _ = journal::revert(&tid);
        return Err(e);
    }
    get(id)
}

/// Apply (or revert) all three per-game tweaks.
pub fn optimize(id: &str, on: bool) -> Result<GameInfo> {
    let mut first_err = None;
    for k in KINDS {
        if let Err(e) = set_tweak(id, k, on) {
            first_err.get_or_insert(e);
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => get(id),
    }
}

/// Add the game to (or remove it from) the Game Mode watcher.
pub fn set_session(id: &str, on: bool) -> Result<GameInfo> {
    let g = get(id)?;
    let exe = g.exe.as_deref().ok_or_else(|| Error::Unavailable("executável do jogo não encontrado".into()))?;
    let file = file_name_lower(exe);
    let mut cfg = crate::session::load_config();
    cfg.games.retain(|s| s.exe != file);
    if on {
        cfg.games.push(crate::session::SessionGame { exe: file, name: g.name.clone(), display: is_cs2(exe) });
    }
    crate::session::set_config(cfg)?;
    get(id)
}

/// Running programs with a window-like footprint, for "add the game that is running".
pub fn running_processes() -> Result<Vec<RunningProcess>> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always).with_memory(),
    );
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let mut v: Vec<RunningProcess> = Vec::new();
    for p in sys.processes().values() {
        let Some(exe) = p.exe() else { continue };
        if detect::is_inside(exe, &windir) || p.memory() < 150 * 1024 * 1024 {
            continue;
        }
        let exe_s = exe.to_string_lossy().to_string();
        if v.iter().any(|r| r.exe.eq_ignore_ascii_case(&exe_s)) {
            continue;
        }
        v.push(RunningProcess { name: p.name().to_string_lossy().to_string(), exe: exe_s, memory_mb: p.memory() / (1024 * 1024) });
    }
    v.sort_by_key(|r| std::cmp::Reverse(r.memory_mb));
    v.truncate(30);
    Ok(v)
}

/// Friendly name for a `game:<id>:<kind>` journal id, for the Restore page.
pub fn describe_tweak_id(tid: &str) -> Option<String> {
    let rest = tid.strip_prefix("game:")?;
    let (gid, kind) = rest.rsplit_once(':')?;
    let name = list(false).ok()?.into_iter().find(|g| g.id == gid).map(|g| g.name).unwrap_or_else(|| gid.to_string());
    let what = match kind {
        "priority" => "prioridade de CPU alta",
        "gpu" => "GPU de alto desempenho",
        "fso" => "sem otimizações de ecrã inteiro",
        _ => return None,
    };
    Some(format!("{name}: {what}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs2_keeps_catalog_ids() {
        assert_eq!(tweak_id("steam:730", r"D:\x\cs2.exe", TweakKind::Priority), "cs2-high-priority");
        assert_eq!(tweak_id("steam:1", r"D:\x\game.exe", TweakKind::Fso), "game:steam:1:fso");
    }

    #[test]
    fn library_survives_garbage() {
        let l: Library = serde_json::from_slice(b"{}").unwrap();
        assert!(l.manual.is_empty() && l.hidden.is_empty());
    }
}
