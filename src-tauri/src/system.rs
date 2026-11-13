//! Hardware/OS inventory and live stats for the dashboard.

use std::sync::Mutex;

use serde::Serialize;
use sysinfo::{ProcessesToUpdate, System};

use crate::error::Result;
use crate::journal::{Hive, RegValue};
use crate::win::registry;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Vendor {
    Amd,
    Intel,
    Nvidia,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub name: String,
    pub vendor: Vendor,
    pub cores: u32,
    pub threads: u32,
    pub zen_gen: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: Vendor,
    pub driver_version: Option<String>,
    pub vram_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os_name: String,
    pub os_build: String,
    pub hostname: String,
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub ram_total_gb: f64,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStats {
    pub cpu_usage: f32,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub process_count: u32,
    pub cs2_running: bool,
    /// Game of the active Game Mode session, if any.
    pub active_game: Option<String>,
}

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

pub fn info() -> Result<SystemInfo> {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let first = sys.cpus().first();
    let name = first.map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let vendor_id = first.map(|c| c.vendor_id().to_string()).unwrap_or_default();
    let vendor = match vendor_id.as_str() {
        "AuthenticAMD" => Vendor::Amd,
        "GenuineIntel" => Vendor::Intel,
        _ => Vendor::Other,
    };
    let threads = sys.cpus().len() as u32;
    let cpu = CpuInfo {
        name,
        vendor,
        cores: System::physical_core_count().map(|c| c as u32).unwrap_or(threads),
        threads,
        zen_gen: if vendor == Vendor::Amd { zen_generation() } else { None },
    };

    Ok(SystemInfo {
        os_name: System::long_os_version().unwrap_or_else(|| "Desconhecido".into()),
        os_build: System::kernel_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
        cpu,
        gpus: gpus(),
        ram_total_gb: round1(sys.total_memory() as f64 / GIB),
        is_admin: is_admin(),
    })
}

/// Persistent System so CPU usage is measured between consecutive calls.
static LIVE: Mutex<Option<System>> = Mutex::new(None);

pub fn live_stats() -> Result<LiveStats> {
    let mut guard = LIVE.lock().unwrap_or_else(|e| e.into_inner());
    let sys = guard.get_or_insert_with(System::new);
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    // Names only: the default refresh also reads CPU, memory, disk and exe path of
    // every process, which is wasted work on a 2 s poll.
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
    let cs2_running = sys.processes().values().any(|p| p.name().eq_ignore_ascii_case("cs2.exe"));
    Ok(LiveStats {
        cpu_usage: sys.global_cpu_usage(),
        ram_used_gb: round1(sys.used_memory() as f64 / GIB),
        ram_total_gb: round1(sys.total_memory() as f64 / GIB),
        process_count: sys.processes().len() as u32,
        cs2_running,
        active_game: crate::session::active_game(),
    })
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Map AMD family/model to a Zen generation.
#[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
pub fn zen_gen_from(family: u32, model: u32) -> Option<u8> {
    match family {
        0x17 if model >= 0x30 => Some(2),
        0x17 => Some(1),
        0x19 => match model {
            0x00..=0x0F | 0x20..=0x2F | 0x40..=0x5F => Some(3),
            0x10..=0x1F | 0x60..=0x7F | 0xA0..=0xAF => Some(4),
            _ => None,
        },
        0x1A => Some(5),
        _ => None,
    }
}

#[cfg(target_arch = "x86_64")]
fn zen_generation() -> Option<u8> {
    // CPUID leaf 1 is available on every x86_64 CPU.
    let r = std::arch::x86_64::__cpuid(1);
    let base_family = (r.eax >> 8) & 0xF;
    let ext_family = (r.eax >> 20) & 0xFF;
    let base_model = (r.eax >> 4) & 0xF;
    let ext_model = (r.eax >> 16) & 0xF;
    let family = if base_family == 0xF { base_family + ext_family } else { base_family };
    let model = (ext_model << 4) | base_model;
    zen_gen_from(family, model)
}

#[cfg(not(target_arch = "x86_64"))]
fn zen_generation() -> Option<u8> {
    None
}

const DISPLAY_CLASS: &str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

/// Display adapters from the device class registry key (one subkey per adapter: 0000, 0001, ...).
fn gpus() -> Vec<GpuInfo> {
    let mut out: Vec<GpuInfo> = Vec::new();
    for sub in registry::subkeys(Hive::Hklm, DISPLAY_CLASS) {
        if !sub.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let key = format!(r"{DISPLAY_CLASS}\{sub}");
        let s = |n: &str| match registry::read(Hive::Hklm, &key, n) {
            Ok(Some(RegValue::Sz(v))) => Some(v),
            _ => None,
        };
        let Some(name) = s("DriverDesc") else { continue };
        // Skip virtual/remote adapters.
        let lower = name.to_lowercase();
        if lower.contains("basic display") || lower.contains("remote") || lower.contains("virtual") {
            continue;
        }
        let vram_mb = match registry::read(Hive::Hklm, &key, "HardwareInformation.qwMemorySize") {
            Ok(Some(RegValue::Qword(b))) => Some(b / (1024 * 1024)),
            Ok(Some(RegValue::Dword(b))) => Some(u64::from(b) / (1024 * 1024)),
            _ => None,
        };
        let vendor = if lower.contains("nvidia") {
            Vendor::Nvidia
        } else if lower.contains("amd") || lower.contains("radeon") {
            Vendor::Amd
        } else if lower.contains("intel") {
            Vendor::Intel
        } else {
            Vendor::Other
        };
        if out.iter().any(|g| g.name == name) {
            continue;
        }
        out.push(GpuInfo { name, vendor, driver_version: s("DriverVersion"), vram_mb });
    }
    out
}

#[cfg(windows)]
pub fn is_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut core::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_admin() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::zen_gen_from;

    #[test]
    fn zen_generations() {
        assert_eq!(zen_gen_from(0x17, 0x71), Some(2)); // Matisse
        assert_eq!(zen_gen_from(0x19, 0x21), Some(3)); // Vermeer
        assert_eq!(zen_gen_from(0x19, 0x50), Some(3)); // Cezanne
        assert_eq!(zen_gen_from(0x19, 0x61), Some(4)); // Raphael
        assert_eq!(zen_gen_from(0x1A, 0x44), Some(5)); // Granite Ridge
    }
}
