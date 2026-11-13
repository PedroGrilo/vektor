//! Windows System Restore points, as an extra safety net on top of the journal.
//! Uses Windows PowerShell 5.1 (`Checkpoint-Computer`), which ships with Windows.

use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePoint {
    pub description: String,
    /// ISO timestamp
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePointResult {
    pub created: bool,
    /// PT-PT explanation (why nothing was created, or what was).
    pub message: String,
    pub last: Option<RestorePoint>,
}

const LAST_SCRIPT: &str = "$p = Get-ComputerRestorePoint -ErrorAction SilentlyContinue | Select-Object -Last 1; \
     if ($p) { [pscustomobject]@{ description = $p.Description; \
     date = ([Management.ManagementDateTimeConverter]::ToDateTime($p.CreationTime)).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress }";

#[cfg(windows)]
fn powershell(script: &str) -> Result<(bool, String, String)> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| Error::msg(format!("Não foi possível executar o PowerShell: {e}")))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    ))
}

#[cfg(not(windows))]
fn powershell(_: &str) -> Result<(bool, String, String)> {
    Err(Error::Unsupported)
}

fn parse_last(json: &str) -> Option<RestorePoint> {
    let v: serde_json::Value = serde_json::from_str(json.trim()).ok()?;
    Some(RestorePoint { description: v["description"].as_str()?.to_string(), date: v["date"].as_str()?.to_string() })
}

/// Most recent restore point, if System Restore has any.
pub fn last() -> Result<Option<RestorePoint>> {
    let (_, out, _) = powershell(LAST_SCRIPT)?;
    Ok(parse_last(&out))
}

/// Create a restore point. With `enable_protection`, System Protection is turned
/// on for the system drive first (it is off on many PCs).
pub fn create(enable_protection: bool) -> Result<RestorePointResult> {
    let before = last()?;
    let enable = if enable_protection { "Enable-ComputerRestore -Drive \"$env:SystemDrive\\\"; " } else { "" };
    let script = format!(
        "$ErrorActionPreference = 'Stop'; {enable}Checkpoint-Computer -Description 'Vektor' -RestorePointType MODIFY_SETTINGS -WarningAction SilentlyContinue"
    );
    let (ok, _, err) = powershell(&script)?;
    let after = last()?;
    let created = after.as_ref().is_some_and(|a| before.as_ref().is_none_or(|b| b.date != a.date));
    let message = if created {
        "Ponto de restauro criado.".to_string()
    } else if !ok && (err.contains("disabled") || err.contains("desativad") || err.contains("0x80042306")) {
        "A Proteção do Sistema está desligada nesta unidade. Ativa-a para criar pontos de restauro.".to_string()
    } else if !ok {
        format!("O Windows recusou criar o ponto de restauro: {}", err.lines().next().unwrap_or("erro desconhecido"))
    } else {
        "O Windows só cria um ponto de restauro por cada 24 horas e já existe um recente. Esse continua válido.".to_string()
    };
    Ok(RestorePointResult { created, message, last: after })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_last_point() {
        let p = parse_last(r#"{"description":"Vektor","date":"2026-11-13T19:00:00.0000000Z"}"#).unwrap();
        assert_eq!(p.description, "Vektor");
        assert!(parse_last("").is_none());
    }
}
