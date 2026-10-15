//! Power plans via powercfg.exe (fixed arguments only).

use crate::error::{Error, Result};
use crate::journal::{self, Undo};

/// Built-in "Ultimate Performance" scheme (hidden on many editions but duplicable).
pub const ULTIMATE_SCHEME: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
/// Built-in "High performance" scheme, fallback when Ultimate cannot be duplicated.
pub const HIGH_PERF_SCHEME: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
/// Fixed GUID for the plan Vektor creates, so it can be found and removed again.
pub const VEKTOR_SCHEME: &str = "7e570b05-7e57-4c2d-9f3a-7e570b057e57";

/// Re-activate `previous_guid`, then delete `created_guid` when Vektor created it.
pub fn restore(previous_guid: &str, created_guid: Option<&str>) -> Result<()> {
    check_guid(previous_guid)?;
    powercfg(&["/setactive", previous_guid])?;
    if let Some(g) = created_guid {
        check_guid(g)?;
        // Deleting a plan that is already gone is fine.
        if scheme_exists(g)? {
            powercfg(&["/delete", g])?;
        }
    }
    Ok(())
}

/// GUID of the currently active scheme.
pub fn active_scheme() -> Result<String> {
    let out = powercfg(&["/getactivescheme"])?;
    find_guid(&out).ok_or_else(|| Error::msg("Não foi possível ler o plano de energia ativo"))
}

pub fn scheme_exists(guid: &str) -> Result<bool> {
    let out = powercfg(&["/list"])?;
    Ok(out.to_lowercase().contains(&guid.to_lowercase()))
}

/// Create (if needed) and activate the Vektor performance plan, journaled under `tweak_id`.
pub fn activate_vektor_plan(tweak_id: &str) -> Result<()> {
    let previous = active_scheme()?;
    let created = if scheme_exists(VEKTOR_SCHEME)? {
        None
    } else {
        if powercfg(&["/duplicatescheme", ULTIMATE_SCHEME, VEKTOR_SCHEME]).is_err() {
            powercfg(&["/duplicatescheme", HIGH_PERF_SCHEME, VEKTOR_SCHEME])?;
        }
        Some(VEKTOR_SCHEME.to_string())
    };
    // Record right after creation so a failure below still cleans the plan up.
    journal::record(tweak_id, Undo::PowerPlan { previous_guid: previous, created_guid: created })?;
    // Name is cosmetic; ignore failures (e.g. on very old builds).
    let _ = powercfg(&["/changename", VEKTOR_SCHEME, "Vektor Performance"]);
    powercfg(&["/setactive", VEKTOR_SCHEME])?;
    Ok(())
}

fn check_guid(g: &str) -> Result<()> {
    if is_guid(g) {
        Ok(())
    } else {
        Err(Error::InvalidInput(format!("GUID inválido: {g}")))
    }
}

fn is_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    parts.len() == 5
        && parts
            .iter()
            .zip(lens)
            .all(|(p, l)| p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// First GUID-shaped token in powercfg output (the text around it is localized).
fn find_guid(text: &str) -> Option<String> {
    text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .find(|t| is_guid(t))
        .map(|t| t.to_lowercase())
}

#[cfg(windows)]
fn powercfg(args: &[&str]) -> Result<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powercfg.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    if out.status.success() {
        Ok(stdout)
    } else {
        Err(Error::msg(format!(
            "powercfg {} falhou: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

#[cfg(not(windows))]
fn powercfg(_args: &[&str]) -> Result<String> {
    Err(Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_guid_from_localized_output() {
        let out = "GUID do Esquema de Energia: 381b4222-f694-41f0-9685-ff5bb260df2e  (Equilibrado)";
        assert_eq!(find_guid(out).as_deref(), Some("381b4222-f694-41f0-9685-ff5bb260df2e"));
        assert!(is_guid(VEKTOR_SCHEME));
        assert!(!is_guid("not-a-guid"));
    }
}
