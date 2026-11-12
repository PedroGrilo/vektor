//! App preferences at `%ProgramData%\Vektor\settings.json`. Autostart is a
//! Task Scheduler logon task with highest privileges, so the elevated app starts
//! without a UAC prompt (a Run key cannot elevate).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::journal;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub autostart: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    /// Look for new releases at startup and every few hours (never installs by itself).
    pub auto_update: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { autostart: false, start_minimized: false, close_to_tray: true, auto_update: true }
    }
}

#[cfg(windows)]
const TASK_NAME: &str = "Vektor";

fn path() -> PathBuf {
    journal::data_dir().join("settings.json")
}

fn parse(bytes: &[u8]) -> AppSettings {
    serde_json::from_slice(bytes).unwrap_or_default()
}

/// Settings file only (no system queries) — cheap, used by lib.rs at startup.
pub fn load() -> AppSettings {
    std::fs::read(path()).map(|b| parse(&b)).unwrap_or_default()
}

/// Settings with `autostart` reflecting the real scheduled task.
pub fn load_checked() -> Result<AppSettings> {
    let mut s = load();
    s.autostart = task_exists();
    Ok(s)
}

pub fn save(s: AppSettings) -> Result<AppSettings> {
    if s.autostart != task_exists() {
        set_autostart(s.autostart)?;
    }
    std::fs::create_dir_all(journal::data_dir())?;
    let tmp = path().with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&s)?)?;
    std::fs::rename(tmp, path())?;
    load_checked()
}

/// `/TR` value: quoted exe path plus the minimized flag. Paths containing a
/// double quote are refused so the command line cannot be broken out of.
#[cfg_attr(not(windows), allow(dead_code))]
fn task_command(exe: &str) -> Option<String> {
    (!exe.contains('"')).then(|| format!("\"{exe}\" --minimized"))
}

#[cfg(windows)]
fn schtasks(args: &[&str]) -> Result<bool> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let sys = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let out = std::process::Command::new(sys.join("System32").join("schtasks.exe"))
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| crate::error::Error::msg(format!("Não foi possível executar o schtasks: {e}")))?;
    Ok(out.status.success())
}

#[cfg(windows)]
fn task_exists() -> bool {
    schtasks(&["/Query", "/TN", TASK_NAME]).unwrap_or(false)
}

#[cfg(not(windows))]
fn task_exists() -> bool {
    false
}

#[cfg(windows)]
fn set_autostart(on: bool) -> Result<()> {
    use crate::error::Error;
    let ok = if on {
        let exe = std::env::current_exe()?;
        let tr = task_command(&exe.to_string_lossy())
            .ok_or_else(|| Error::InvalidInput("caminho do executável inválido".into()))?;
        schtasks(&["/Create", "/TN", TASK_NAME, "/TR", &tr, "/SC", "ONLOGON", "/RL", "HIGHEST", "/F"])?
    } else {
        schtasks(&["/Delete", "/TN", TASK_NAME, "/F"])?
    };
    if ok {
        Ok(())
    } else {
        Err(Error::msg(if on {
            "Não foi possível criar a tarefa de arranque automático"
        } else {
            "Não foi possível remover a tarefa de arranque automático"
        }))
    }
}

#[cfg(not(windows))]
fn set_autostart(_on: bool) -> Result<()> {
    Err(crate::error::Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_partial_file() {
        assert_eq!(parse(b"garbage"), AppSettings::default());
        let s = parse(br#"{"startMinimized":true}"#);
        assert!(s.start_minimized && s.close_to_tray && !s.autostart && s.auto_update);
    }

    #[test]
    fn task_command_quotes_path() {
        assert_eq!(
            task_command(r"C:\Program Files\Vektor\vektor.exe").as_deref(),
            Some(r#""C:\Program Files\Vektor\vektor.exe" --minimized"#)
        );
        assert!(task_command(r#"C:\a"b\vektor.exe"#).is_none());
    }
}
