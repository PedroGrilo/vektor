//! Rollback journal: every change Vektor makes is recorded here *before* it is
//! applied, so it can be undone even after a crash or reboot.
//! Stored at `%ProgramData%\Vektor\journal.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Hive {
    Hkcu,
    Hklm,
}

/// Registry value snapshot (only the types Vektor writes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum RegValue {
    Dword(u32),
    Qword(u64),
    Sz(String),
}

/// One reversible step. Undo runs in reverse order of recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Undo {
    /// Restore `previous` or delete the value when it did not exist.
    RegistryValue { hive: Hive, key: String, name: String, previous: Option<RegValue> },
    /// Re-activate `previous_guid`, delete `created_guid` if Vektor created it.
    PowerPlan { previous_guid: String, created_guid: Option<String> },
    /// Restore service start type and running state.
    Service { name: String, previous_start: u32, was_running: bool },
    /// Restore file from `backup`, or delete `path` when `backup` is None (file was created).
    File { path: PathBuf, backup: Option<PathBuf> },
    /// Restore the read-only attribute.
    FileReadonly { path: PathBuf, was_readonly: bool },
    /// Re-import the saved NVIDIA "Base Profile" snapshot (.nip) in replace mode.
    NvidiaBaseProfile { snapshot: PathBuf },
    /// Stop and delete the VektorMemory service.
    MemoryService,
    /// Restore a display mode changed for the game session.
    DisplayMode { device: String, width: u32, height: u32, refresh: u32 },
    /// Restore digital vibrance (0..100 scale) on a display.
    Vibrance { device: String, previous: i32 },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct JournalFile {
    /// tweak id -> undo steps in application order
    entries: BTreeMap<String, Vec<Undo>>,
}

static LOCK: Mutex<()> = Mutex::new(());

pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        base.join("Vektor")
    }
    #[cfg(not(windows))]
    {
        std::env::temp_dir().join("vektor")
    }
}

/// Directory for file backups made before overwriting user files.
pub fn backup_dir() -> PathBuf {
    data_dir().join("backups")
}

fn path() -> PathBuf {
    data_dir().join("journal.json")
}

fn load() -> Result<JournalFile> {
    match std::fs::read(path()) {
        Ok(b) => Ok(serde_json::from_slice(&b)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(JournalFile::default()),
        Err(e) => Err(e.into()),
    }
}

fn save(j: &JournalFile) -> Result<()> {
    std::fs::create_dir_all(data_dir())?;
    let tmp = path().with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(j)?)?;
    std::fs::rename(tmp, path())?;
    Ok(())
}

/// Record an undo step for `tweak_id`. Call this BEFORE making the change.
pub fn record(tweak_id: &str, undo: Undo) -> Result<()> {
    let _g = LOCK.lock().unwrap();
    let mut j = load()?;
    j.entries.entry(tweak_id.to_string()).or_default().push(undo);
    save(&j)
}

/// Drop the entries for `tweak_id` without running them (the change was undone by hand).
pub fn forget(tweak_id: &str) -> Result<()> {
    let _g = LOCK.lock().unwrap();
    let mut j = load()?;
    if j.entries.remove(tweak_id).is_some() {
        save(&j)?;
    }
    Ok(())
}

pub fn is_applied(tweak_id: &str) -> bool {
    let _g = LOCK.lock().unwrap();
    load().map(|j| j.entries.contains_key(tweak_id)).unwrap_or(false)
}

pub fn applied_ids() -> Vec<String> {
    let _g = LOCK.lock().unwrap();
    load().map(|j| j.entries.keys().cloned().collect()).unwrap_or_default()
}

/// Undo every step recorded for `tweak_id` (reverse order). Steps that fail are
/// kept in the journal so a later retry can finish the job.
pub fn revert(tweak_id: &str) -> Result<()> {
    let steps = {
        let _g = LOCK.lock().unwrap();
        let j = load()?;
        j.entries.get(tweak_id).cloned().unwrap_or_default()
    };
    let mut failed = Vec::new();
    let mut first_err = None;
    for step in steps.into_iter().rev() {
        if let Err(e) = crate::undo::run(&step) {
            failed.insert(0, step);
            first_err.get_or_insert(e);
        }
    }
    let _g = LOCK.lock().unwrap();
    let mut j = load()?;
    if failed.is_empty() {
        j.entries.remove(tweak_id);
    } else {
        j.entries.insert(tweak_id.to_string(), failed);
    }
    save(&j)?;
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub fn revert_all() -> Result<()> {
    let mut first_err = None;
    for id in applied_ids().into_iter().rev() {
        if let Err(e) = revert(&id) {
            first_err.get_or_insert(e);
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
