//! Closing allowlisted apps for the session and reopening them afterwards.

use std::path::{Path, PathBuf};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::tweaks::processes;

pub(crate) struct Closed {
    pub label: String,
    /// Executable to relaunch (one per app, not per process).
    pub exe: Option<PathBuf>,
}

/// Terminate every running process whose name is in `names` (already filtered
/// against the allowlist). Returns one entry per distinct app closed.
pub(crate) fn close(names: &[String]) -> Vec<Closed> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    let mut out: Vec<Closed> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for p in sys.processes().values() {
        let name = p.name().to_string_lossy().to_lowercase();
        if !names.contains(&name) || !processes::is_allowed(&name) {
            continue;
        }
        let exe = p.exe().map(Path::to_path_buf);
        if !p.kill() {
            continue;
        }
        if !seen.contains(&name) {
            seen.push(name.clone());
            out.push(Closed { label: processes::label(&name).unwrap_or("app").to_string(), exe });
        }
    }
    out
}

/// Relaunch executables in the interactive user's (non-elevated) context.
/// Vektor runs elevated; starting a program through Explorer makes the shell
/// launch it with the user's normal token. Returns the labels reopened.
pub(crate) fn reopen(paths: &[PathBuf]) -> Vec<String> {
    let mut done = Vec::new();
    for p in paths {
        let Some(name) = p.file_name().map(|n| n.to_string_lossy().to_lowercase()) else { continue };
        // Defence in depth: the state file is on disk, only relaunch allowlisted apps.
        if !processes::is_allowed(&name) || !p.is_file() {
            continue;
        }
        if launch_unelevated(p) {
            done.push(processes::label(&name).unwrap_or("app").to_string());
        }
    }
    done
}

#[cfg(windows)]
fn launch_unelevated(path: &Path) -> bool {
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    std::process::Command::new(windir.join("explorer.exe")).arg(path).spawn().is_ok()
}

#[cfg(not(windows))]
fn launch_unelevated(_path: &Path) -> bool {
    false
}
