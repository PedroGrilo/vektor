//! NVIDIA global ("Base Profile") optimisation through NVIDIA Profile Inspector
//! (MIT, github.com/Orbmu2k/nvidiaProfileInspector), bundled under `resources/npi/`.
//!
//! Flow:
//! 1. `-exportCustomized` → snapshot of the current global profile (for rollback).
//! 2. `-silentImport -mergeImport vektor-global.nip` → our settings are merged into
//!    the current global profile (empty ProfileName = current global profile).
//! 3. Export again and check our setting IDs are present (silent import hides errors).
//!
//! Revert re-imports the snapshot with `-replaceImport`, which resets the global
//! profile and writes back exactly the values it had before.
//!
//! Setting IDs/values come from NVIDIA's public NvApiDriverSettings.h.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::error::{Error, Result};
use crate::journal::{self, Undo};

const TWEAK_ID: &str = "nvidia-global";
const NPI_EXE: &str = "nvidiaProfileInspector.exe";
const NPI_VERSION: &str = "3.0.2.1";

const PRERENDERLIMIT_ID: u32 = 0x007B_A09E;
const PREFERRED_PSTATE_ID: u32 = 0x1057_EB71;
const PREFERRED_PSTATE_PREFER_MAX: u32 = 0x1;
const QUALITY_ENHANCEMENTS_ID: u32 = 0x00CE_2691;
const QUALITY_ENHANCEMENTS_HIGHPERFORMANCE: u32 = 0x14;
const PS_TEXFILTER_BILINEAR_IN_ANISO_ID: u32 = 0x0084_CD70;
const PS_TEXFILTER_DISABLE_TRILIN_SLOPE_ID: u32 = 0x002E_CAF2;
const PS_SHADERDISKCACHE_ID: u32 = 0x0019_8FFF;
const PS_SHADERDISKCACHE_MAX_SIZE_ID: u32 = 0x00AC_8497;
const PS_SHADERDISKCACHE_MAX_SIZE_UNLIMITED: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvGlobalOptions {
    pub max_performance_power: bool,
    pub low_latency: bool,
    pub texture_high_performance: bool,
    pub unlimited_shader_cache: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaStatus {
    pub nvidia_gpu: bool,
    pub inspector_available: bool,
    pub applied: bool,
    pub inspector_version: Option<String>,
}

struct Setting {
    id: u32,
    value: u32,
    name: &'static str,
}

fn settings_for(o: &NvGlobalOptions) -> Vec<Setting> {
    let mut v = Vec::new();
    if o.max_performance_power {
        v.push(Setting { id: PREFERRED_PSTATE_ID, value: PREFERRED_PSTATE_PREFER_MAX, name: "Power management mode" });
    }
    if o.low_latency {
        v.push(Setting { id: PRERENDERLIMIT_ID, value: 1, name: "Maximum pre-rendered frames" });
    }
    if o.texture_high_performance {
        v.push(Setting { id: QUALITY_ENHANCEMENTS_ID, value: QUALITY_ENHANCEMENTS_HIGHPERFORMANCE, name: "Texture filtering - Quality" });
        v.push(Setting { id: PS_TEXFILTER_BILINEAR_IN_ANISO_ID, value: 1, name: "Texture filtering - Anisotropic sample optimization" });
        v.push(Setting { id: PS_TEXFILTER_DISABLE_TRILIN_SLOPE_ID, value: 1, name: "Texture filtering - Trilinear optimization" });
    }
    if o.unlimited_shader_cache {
        v.push(Setting { id: PS_SHADERDISKCACHE_ID, value: 1, name: "Shader Cache" });
        v.push(Setting { id: PS_SHADERDISKCACHE_MAX_SIZE_ID, value: PS_SHADERDISKCACHE_MAX_SIZE_UNLIMITED, name: "Shader disk cache maximum size" });
    }
    v
}

// ---------- .nip (XmlSerializer of List<Profile>) ----------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn render_global_nip(settings: &[Setting]) -> String {
    let mut body = String::new();
    for s in settings {
        body.push_str(&format!(
            "      <ProfileSetting>\r\n        <SettingNameInfo>{}</SettingNameInfo>\r\n        <SettingID>{}</SettingID>\r\n        <SettingValue>{}</SettingValue>\r\n        <ValueType>Dword</ValueType>\r\n      </ProfileSetting>\r\n",
            xml_escape(s.name), s.id, s.value
        ));
    }
    wrap_profiles(&format!(
        "  <Profile>\r\n    <ProfileName />\r\n    <Executeables />\r\n    <Settings>\r\n{body}    </Settings>\r\n  </Profile>\r\n"
    ))
}

fn wrap_profiles(profiles: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<ArrayOfProfile>\r\n{profiles}</ArrayOfProfile>\r\n")
}

/// Decode an NPI export (UTF-16 LE/BE or UTF-8, with or without BOM).
fn decode_nip(bytes: &[u8]) -> String {
    let utf16 = |be: bool, b: &[u8]| {
        let units: Vec<u16> = b
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| if be { u16::from_be_bytes(*c) } else { u16::from_le_bytes(*c) })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(false, rest),
        [0xFE, 0xFF, rest @ ..] => utf16(true, rest),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Extract the `<Profile>` block of the global profile from an export, retargeted
/// to the current global profile (empty ProfileName).
fn extract_global_profile(xml: &str, global_name: &str) -> Option<String> {
    let mut rest = xml;
    while let Some(start) = rest.find("<Profile>") {
        let after = &rest[start..];
        let end = after.find("</Profile>")? + "</Profile>".len();
        let block = &after[..end];
        let wanted = format!("<ProfileName>{}</ProfileName>", xml_escape(global_name));
        if block.contains(&wanted) {
            return Some(format!("  {}\r\n", block.replacen(&wanted, "<ProfileName />", 1)));
        }
        rest = &after[end..];
    }
    None
}

/// Snapshot used when the global profile had no customisations: replace-importing
/// it resets the global profile to driver defaults.
fn empty_global_profile() -> String {
    "  <Profile>\r\n    <ProfileName />\r\n    <Executeables />\r\n    <Settings />\r\n  </Profile>\r\n".to_string()
}

fn global_contains(block: &str, settings: &[Setting]) -> bool {
    settings.iter().all(|s| {
        let id = format!("<SettingID>{}</SettingID>", s.id);
        let val = format!("<SettingValue>{}</SettingValue>", s.value);
        block.split("<ProfileSetting>").any(|ps| ps.contains(&id) && ps.contains(&val))
    })
}

// ---------- paths & process ----------

fn npi_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = app.path().resource_dir().ok()?.join("resources").join("npi");
    dir.join(NPI_EXE).is_file().then_some(dir)
}

fn work_dir() -> PathBuf {
    journal::data_dir().join("nvidia")
}

fn has_nvidia_gpu() -> bool {
    crate::system::info()
        .map(|i| i.gpus.iter().any(|g| g.vendor == crate::system::Vendor::Nvidia))
        .unwrap_or(false)
}

#[cfg(windows)]
fn run_npi(dir: &Path, args: &[&std::ffi::OsStr]) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let status = std::process::Command::new(dir.join(NPI_EXE))
        .args(args)
        .current_dir(dir)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| Error::msg(format!("Não foi possível executar o Profile Inspector: {e}")))?;
    if !status.success() {
        return Err(Error::msg(format!("O Profile Inspector terminou com erro ({status})")));
    }
    Ok(())
}

#[cfg(not(windows))]
fn run_npi(_dir: &Path, _args: &[&std::ffi::OsStr]) -> Result<()> {
    Err(Error::Unsupported)
}

/// Run `-exportCustomized` and return the decoded export (the file is removed).
fn export_customized(dir: &Path) -> Result<String> {
    let before = std::time::SystemTime::now();
    run_npi(dir, &["-exportCustomized".as_ref()])?;
    let newest = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("CustomProfiles_") && n.ends_with(".nip")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(t, _)| *t >= before - std::time::Duration::from_secs(2))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
        .ok_or_else(|| Error::msg("O Profile Inspector não exportou o perfil atual (driver NVIDIA instalado?)"))?;
    let xml = decode_nip(&std::fs::read(&newest)?);
    let _ = std::fs::remove_file(&newest);
    Ok(xml)
}

fn import(dir: &Path, nip: &Path, mode: &str) -> Result<()> {
    run_npi(dir, &["-silentImport".as_ref(), mode.as_ref(), nip.as_os_str()])
}

// ---------- public API ----------

pub fn status(app: &tauri::AppHandle) -> Result<NvidiaStatus> {
    let available = npi_dir(app).is_some();
    Ok(NvidiaStatus {
        nvidia_gpu: has_nvidia_gpu(),
        inspector_available: available,
        applied: journal::is_applied(TWEAK_ID),
        inspector_version: available.then(|| NPI_VERSION.to_string()),
    })
}

pub fn apply_global(app: &tauri::AppHandle, options: &NvGlobalOptions) -> Result<NvidiaStatus> {
    if !has_nvidia_gpu() {
        return Err(Error::Unavailable("não foi encontrada uma placa gráfica NVIDIA".into()));
    }
    let dir = npi_dir(app).ok_or_else(|| Error::Unavailable("NVIDIA Profile Inspector não incluído nesta instalação".into()))?;
    let settings = settings_for(options);
    if settings.is_empty() {
        return Err(Error::InvalidInput("escolhe pelo menos uma opção".into()));
    }
    std::fs::create_dir_all(work_dir())?;

    // Snapshot only on first application, so revert always returns to the original state.
    if !journal::is_applied(TWEAK_ID) {
        let export = export_customized(&dir)?;
        let block = extract_global_profile(&export, "Base Profile").unwrap_or_else(empty_global_profile);
        let snapshot = work_dir().join("global-before-vektor.nip");
        std::fs::write(&snapshot, format!("\u{feff}{}", wrap_profiles(&block)))?;
        journal::record(TWEAK_ID, Undo::NvidiaBaseProfile { snapshot })?;
    }

    let nip = work_dir().join("vektor-global.nip");
    std::fs::write(&nip, format!("\u{feff}{}", render_global_nip(&settings)))?;
    import(&dir, &nip, "-mergeImport")?;

    let after = export_customized(&dir)?;
    let ok = extract_global_profile(&after, "Base Profile").is_some_and(|b| global_contains(&b, &settings));
    if !ok {
        return Err(Error::msg(
            "O perfil foi importado mas as definições não aparecem no perfil global. Verifica no Profile Inspector.",
        ));
    }
    status(app)
}

pub fn revert_global(app: &tauri::AppHandle) -> Result<NvidiaStatus> {
    journal::revert(TWEAK_ID)?;
    status(app)
}

/// Undo step: replace-import the saved global profile snapshot.
pub fn restore_base_profile(snapshot: &Path) -> Result<()> {
    // The undo runner has no AppHandle; NPI lives next to the executable in installs.
    let dir = std::env::current_exe()?
        .parent()
        .map(|p| p.join("resources").join("npi"))
        .filter(|d| d.join(NPI_EXE).is_file())
        .ok_or_else(|| Error::Unavailable("NVIDIA Profile Inspector não encontrado".into()))?;
    if !snapshot.is_file() {
        return Err(Error::msg("cópia de segurança do perfil NVIDIA em falta"));
    }
    import(&dir, snapshot, "-replaceImport")
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<ArrayOfProfile>\r\n  <Profile>\r\n    <ProfileName>Counter-strike 2</ProfileName>\r\n    <Settings />\r\n  </Profile>\r\n  <Profile>\r\n    <ProfileName>Base Profile</ProfileName>\r\n    <Executeables />\r\n    <Settings>\r\n      <ProfileSetting>\r\n        <SettingNameInfo>Maximum pre-rendered frames</SettingNameInfo>\r\n        <SettingID>8102046</SettingID>\r\n        <SettingValue>1</SettingValue>\r\n        <ValueType>Dword</ValueType>\r\n      </ProfileSetting>\r\n    </Settings>\r\n  </Profile>\r\n</ArrayOfProfile>";

    #[test]
    fn extracts_and_retargets_global() {
        let b = extract_global_profile(EXPORT, "Base Profile").unwrap();
        assert!(b.contains("<ProfileName />"));
        assert!(b.contains("<SettingID>8102046</SettingID>"));
        assert!(!b.contains("Counter-strike"));
    }

    #[test]
    fn missing_global_is_none() {
        assert!(extract_global_profile("<ArrayOfProfile></ArrayOfProfile>", "Base Profile").is_none());
    }

    #[test]
    fn decodes_utf16_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for u in "<a>ç</a>".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode_nip(&bytes), "<a>ç</a>");
    }

    #[test]
    fn render_and_verify_roundtrip() {
        let o = NvGlobalOptions { max_performance_power: true, low_latency: true, texture_high_performance: true, unlimited_shader_cache: true };
        let s = settings_for(&o);
        assert_eq!(s.len(), 7);
        let nip = render_global_nip(&s);
        assert!(nip.contains("<SettingID>8102046</SettingID>")); // PRERENDERLIMIT 0x007BA09E
        assert!(nip.contains("<SettingValue>4294967295</SettingValue>"));
        assert!(global_contains(&nip, &s));
        assert!(!global_contains(EXPORT, &s));
    }
}
