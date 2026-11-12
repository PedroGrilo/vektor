//! ISLC-style standby list cleaner.
//!
//! The same executable runs as the `VektorMemory` Windows service when launched
//! with [`SERVICE_ARG`]. There is no IPC: the app writes `memory.json`, the
//! service reads it every tick (treating it as untrusted, clamped input) and
//! reports counters through `memory-status.json`.

mod nt;
pub mod service;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Journal id so "Reverter tudo" also removes the service.
const SERVICE_TWEAK_ID: &str = "memory-service";

pub const SERVICE_ARG: &str = "--memory-service";
#[cfg_attr(not(windows), allow(dead_code))]
pub const SERVICE_NAME: &str = "VektorMemory";
#[cfg_attr(not(windows), allow(dead_code))]
pub const SERVICE_DISPLAY: &str = "Vektor Memory Cleaner";
#[cfg_attr(not(windows), allow(dead_code))]
pub const SERVICE_DESCRIPTION: &str =
    "Limpa a lista de memória em espera (standby) quando a memória livre fica baixa, para reduzir stutters em jogos.";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConfig {
    pub enabled: bool,
    pub standby_threshold_mb: u64,
    pub free_threshold_mb: u64,
    pub interval_sec: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { enabled: true, standby_threshold_mb: 1024, free_threshold_mb: 1024, interval_sec: 5 }
    }
}

impl MemoryConfig {
    pub fn clamped(mut self) -> Self {
        self.standby_threshold_mb = self.standby_threshold_mb.clamp(256, 65536);
        self.free_threshold_mb = self.free_threshold_mb.clamp(256, 65536);
        self.interval_sec = self.interval_sec.clamp(2, 60);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    pub service_installed: bool,
    pub service_running: bool,
    pub config: MemoryConfig,
    pub total_mb: u64,
    pub free_mb: u64,
    pub standby_mb: u64,
    pub modified_mb: u64,
    pub purge_count: u64,
    pub last_purge: Option<String>,
    pub last_check: Option<String>,
    pub last_error: Option<String>,
}

/// State reported by the service: purge counters, a heartbeat and the last failure.
/// Without the last two a service that cannot read memory or purge looks identical
/// to one that simply has nothing to do.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ServiceCounters {
    pub purge_count: u64,
    pub last_purge: Option<String>,
    /// Last time the service evaluated the rule.
    pub last_check: Option<String>,
    /// Why the last evaluation or purge failed; cleared on the next success.
    pub last_error: Option<String>,
}

fn config_path() -> PathBuf {
    crate::journal::data_dir().join("memory.json")
}

fn counters_path() -> PathBuf {
    crate::journal::data_dir().join("memory-status.json")
}

/// Parse untrusted config bytes; anything invalid falls back to defaults.
pub(crate) fn parse_config(bytes: &[u8]) -> MemoryConfig {
    serde_json::from_slice::<MemoryConfig>(bytes).unwrap_or_default().clamped()
}

pub(crate) fn load_config() -> MemoryConfig {
    std::fs::read(config_path()).map(|b| parse_config(&b)).unwrap_or_default()
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(crate::journal::data_dir())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub(crate) fn load_counters() -> ServiceCounters {
    std::fs::read(counters_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn save_counters(c: &ServiceCounters) -> Result<()> {
    write_atomic(&counters_path(), &serde_json::to_vec_pretty(c)?)
}

/// Current UTC time as ISO 8601 (`2026-10-15T18:30:00Z`) without extra crates.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso8601_from_unix(secs)
}

fn iso8601_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

// ---------- app side ----------

pub fn status() -> Result<MemoryStatus> {
    let (installed, running) = service::query()?;
    let lists = nt::memory_lists()?;
    let counters = load_counters();
    Ok(MemoryStatus {
        service_installed: installed,
        service_running: running,
        config: load_config(),
        total_mb: total_memory_mb(),
        free_mb: lists.free_bytes / MB,
        standby_mb: lists.standby_bytes / MB,
        modified_mb: lists.modified_bytes / MB,
        purge_count: counters.purge_count,
        last_purge: counters.last_purge,
        // Heartbeat and error describe a live service; stale ones would mislead.
        last_check: counters.last_check.filter(|_| running),
        last_error: counters.last_error.filter(|_| running),
    })
}

pub fn set_config(c: &MemoryConfig) -> Result<MemoryStatus> {
    let c = c.clone().clamped();
    write_atomic(&config_path(), &serde_json::to_vec_pretty(&c)?)?;
    status()
}

pub fn install_service() -> Result<MemoryStatus> {
    // Make sure the service starts with a valid config file.
    if !config_path().exists() {
        write_atomic(&config_path(), &serde_json::to_vec_pretty(&MemoryConfig::default())?)?;
    }
    let already = service::query()?.0;
    if !already {
        crate::journal::record(SERVICE_TWEAK_ID, crate::journal::Undo::MemoryService)?;
    }
    service::install()?;
    status()
}

pub fn uninstall_service() -> Result<MemoryStatus> {
    service::uninstall()?;
    // Uninstalled by hand: nothing left to roll back.
    crate::journal::forget(SERVICE_TWEAK_ID)?;
    status()
}

/// Remove the service if present; used by the rollback journal. Missing service is fine.
pub fn uninstall_service_quiet() -> Result<()> {
    match service::query()? {
        (false, _) => Ok(()),
        _ => service::uninstall(),
    }
}

pub fn purge_now() -> Result<MemoryStatus> {
    nt::purge_standby_list()?;
    status()
}

const MB: u64 = 1024 * 1024;

fn total_memory_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / MB
}

/// Purge decision shared by the service loop (kept pure for tests).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn should_purge(cfg: &MemoryConfig, standby_bytes: u64, free_bytes: u64) -> bool {
    cfg.enabled
        && standby_bytes >= cfg.standby_threshold_mb * MB
        && free_bytes < cfg.free_threshold_mb * MB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_config_falls_back_to_defaults() {
        assert_eq!(parse_config(b"not json"), MemoryConfig::default());
        assert_eq!(parse_config(br#"{"enabled":"yes"}"#), MemoryConfig::default());
    }

    #[test]
    fn config_is_clamped() {
        let c = parse_config(
            br#"{"enabled":true,"standbyThresholdMb":1,"freeThresholdMb":999999,"intervalSec":0}"#,
        );
        assert_eq!(c.standby_threshold_mb, 256);
        assert_eq!(c.free_threshold_mb, 65536);
        assert_eq!(c.interval_sec, 2);
    }

    #[test]
    fn purge_rule() {
        let c = MemoryConfig::default();
        assert!(should_purge(&c, 2048 * MB, 512 * MB));
        assert!(!should_purge(&c, 512 * MB, 512 * MB));
        assert!(!should_purge(&c, 2048 * MB, 4096 * MB));
        let off = MemoryConfig { enabled: false, ..c };
        assert!(!should_purge(&off, 2048 * MB, 512 * MB));
    }

    #[test]
    fn iso_timestamp() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_from_unix(1_792_085_064), "2026-10-15T17:24:24Z");
    }
}
