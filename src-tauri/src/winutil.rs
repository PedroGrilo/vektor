//! Chris Titus Tech's WinUtil (https://christitus.com/win), opened on request in
//! its own PowerShell window. Vektor is elevated, so the window is too, which is
//! what WinUtil needs. The script is downloaded fresh from christitus.com each
//! time; Vektor does not ship or check it.

use crate::error::Result;

pub const SCRIPT_URL: &str = "https://christitus.com/win";

#[cfg(windows)]
pub fn open() -> Result<()> {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;

    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    // Full path so a powershell.exe elsewhere on PATH is never picked up.
    let root = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let ps = root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    std::process::Command::new(ps)
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &format!("irm {SCRIPT_URL} | iex")])
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn open() -> Result<()> {
    Err(crate::error::Error::Unsupported)
}
