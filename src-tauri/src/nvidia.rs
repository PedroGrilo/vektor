//! NVIDIA driver profile optimisation through NVIDIA Profile Inspector
//! (MIT, github.com/Orbmu2k/nvidiaProfileInspector), bundled under `resources/npi/`.
//! Two targets: the global profile (all games) and the predefined CS2 profile.
//!
//! Flow:
//! 1. `-exportCustomized` → snapshot of the current global profile (for rollback).
//! 2. `-silentImport -mergeImport vektor-global.nip` → our settings are merged into
//!    the current global profile (empty ProfileName = current global profile).
//! 3. Export again and check our setting IDs are present (silent import hides errors).
//!
//! Revert re-imports the snapshot with `-replaceImport`, which resets the profile
//! and writes back exactly the values it had before. Replace mode also deletes
//! applications missing from the import, so snapshots always carry the profile's
//! full application list (merge mode, used to apply, only adds).
//!
//! Setting IDs/values come from NVIDIA's public NvApiDriverSettings.h.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::error::{Error, Result};
use crate::journal::{self, Undo};

/// One profile Vektor can optimise.
struct Target {
    tweak_id: &'static str,
    /// Name used in our .nip ("" = current global profile).
    import_name: &'static str,
    /// Name the profile has in NPI exports.
    export_name: &'static str,
    /// Executables added (merge) so a wrong profile name fails instead of creating an orphan profile.
    executables: &'static [&'static str],
    file_stem: &'static str,
}

const GLOBAL: Target = Target {
    tweak_id: "nvidia-global",
    import_name: "",
    export_name: "Base Profile",
    executables: &[],
    file_stem: "global",
};

const CS2: Target = Target {
    tweak_id: "nvidia-cs2",
    import_name: "Counter-Strike 2",
    export_name: "Counter-Strike 2",
    executables: &["cs2.exe"],
    file_stem: "cs2",
};
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
const VSYNCMODE_ID: u32 = 0x00A8_79CF;
const VSYNCMODE_FORCEOFF: u32 = 0x0841_6747;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvGlobalOptions {
    pub max_performance_power: bool,
    pub low_latency: bool,
    pub texture_high_performance: bool,
    pub unlimited_shader_cache: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NvCs2Options {
    pub low_latency: bool,
    pub max_performance_power: bool,
    pub vsync_off: bool,
    pub texture_high_performance: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaStatus {
    pub nvidia_gpu: bool,
    pub inspector_available: bool,
    pub applied: bool,
    pub cs2_applied: bool,
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

fn settings_for_cs2(o: &NvCs2Options) -> Vec<Setting> {
    let base = NvGlobalOptions {
        max_performance_power: o.max_performance_power,
        low_latency: o.low_latency,
        texture_high_performance: o.texture_high_performance,
        unlimited_shader_cache: false,
    };
    let mut v = settings_for(&base);
    if o.vsync_off {
        v.push(Setting { id: VSYNCMODE_ID, value: VSYNCMODE_FORCEOFF, name: "Vertical Sync" });
    }
    v
}

// ---------- .nip (XmlSerializer of List<Profile>) ----------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn render_nip(target: &Target, settings: &[Setting]) -> String {
    let mut body = String::new();
    for s in settings {
        body.push_str(&format!(
            "      <ProfileSetting>\r\n        <SettingNameInfo>{}</SettingNameInfo>\r\n        <SettingID>{}</SettingID>\r\n        <SettingValue>{}</SettingValue>\r\n        <ValueType>Dword</ValueType>\r\n      </ProfileSetting>\r\n",
            xml_escape(s.name), s.id, s.value
        ));
    }
    wrap_profiles(&format!(
        "  <Profile>\r\n    {}\r\n    {}\r\n    <Settings>\r\n{body}    </Settings>\r\n  </Profile>\r\n",
        profile_name_xml(target.import_name),
        executables_xml(target.executables),
    ))
}

fn profile_name_xml(name: &str) -> String {
    if name.is_empty() { "<ProfileName />".into() } else { format!("<ProfileName>{}</ProfileName>", xml_escape(name)) }
}

fn executables_xml(exes: &[&str]) -> String {
    if exes.is_empty() {
        return "<Executeables />".into();
    }
    let items: String = exes.iter().map(|e| format!("<string>{}</string>", xml_escape(e))).collect();
    format!("<Executeables>{items}</Executeables>")
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

/// Extract the `<Profile>` block named `export_name` (case-insensitive) from an
/// export, renamed to `import_name` so it can be re-imported.
fn extract_profile(xml: &str, export_name: &str, import_name: &str) -> Option<String> {
    let mut rest = xml;
    while let Some(start) = rest.find("<Profile>") {
        let after = &rest[start..];
        let end = after.find("</Profile>")? + "</Profile>".len();
        let block = &after[..end];
        if let (Some(a), Some(b)) = (block.find("<ProfileName>"), block.find("</ProfileName>")) {
            let name = &block[a + "<ProfileName>".len()..b];
            if name.eq_ignore_ascii_case(&xml_escape(export_name)) {
                let full = &block[a..b + "</ProfileName>".len()];
                return Some(format!("  {}\r\n", block.replacen(full, &profile_name_xml(import_name), 1)));
            }
        }
        rest = &after[end..];
    }
    None
}

/// Same profile with its settings removed (applications kept): replace-importing
/// it resets the profile to driver defaults without touching its executables.
fn strip_settings(block: &str) -> String {
    match (block.find("<Settings>"), block.find("</Settings>")) {
        (Some(a), Some(b)) => format!("{}<Settings />{}", &block[..a], &block[b + "</Settings>".len()..]),
        _ => block.to_string(),
    }
}

fn profile_contains(block: &str, settings: &[Setting]) -> bool {
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
        applied: journal::is_applied(GLOBAL.tweak_id),
        cs2_applied: journal::is_applied(CS2.tweak_id),
        inspector_version: available.then(|| NPI_VERSION.to_string()),
    })
}

pub fn apply_global(app: &tauri::AppHandle, options: &NvGlobalOptions) -> Result<NvidiaStatus> {
    apply(app, &GLOBAL, &settings_for(options))
}

pub fn revert_global(app: &tauri::AppHandle) -> Result<NvidiaStatus> {
    journal::revert(GLOBAL.tweak_id)?;
    status(app)
}

pub fn apply_cs2(app: &tauri::AppHandle, options: &NvCs2Options) -> Result<NvidiaStatus> {
    apply(app, &CS2, &settings_for_cs2(options))
}

pub fn revert_cs2(app: &tauri::AppHandle) -> Result<NvidiaStatus> {
    journal::revert(CS2.tweak_id)?;
    status(app)
}

fn apply(app: &tauri::AppHandle, target: &Target, settings: &[Setting]) -> Result<NvidiaStatus> {
    if !has_nvidia_gpu() {
        return Err(Error::Unavailable("não foi encontrada uma placa gráfica NVIDIA".into()));
    }
    let dir = npi_dir(app).ok_or_else(|| Error::Unavailable("NVIDIA Profile Inspector não incluído nesta instalação".into()))?;
    if settings.is_empty() {
        return Err(Error::InvalidInput("escolhe pelo menos uma opção".into()));
    }
    std::fs::create_dir_all(work_dir())?;
    let snapshot = work_dir().join(format!("{}-before-vektor.nip", target.file_stem));
    let first_time = !journal::is_applied(target.tweak_id);

    // Snapshot only on first application, so revert always returns to the original state.
    // If the profile had no customisations it is not in the export yet; the snapshot
    // is then taken from the post-apply export with settings stripped.
    let mut need_snapshot = false;
    if first_time {
        let before = export_customized(&dir)?;
        match extract_profile(&before, target.export_name, target.import_name) {
            Some(block) => save_snapshot(target, &snapshot, &block)?,
            None => need_snapshot = true,
        }
    }

    let nip = work_dir().join(format!("vektor-{}.nip", target.file_stem));
    std::fs::write(&nip, format!("\u{feff}{}", render_nip(target, settings)))?;
    import(&dir, &nip, "-mergeImport")?;

    let after = export_customized(&dir)?;
    let block = extract_profile(&after, target.export_name, target.import_name);
    if need_snapshot {
        if let Some(b) = &block {
            save_snapshot(target, &snapshot, &strip_settings(b))?;
        }
    }
    if !block.is_some_and(|b| profile_contains(&b, settings)) {
        return Err(Error::msg(format!(
            "As definições não aparecem no perfil \"{}\" depois da importação. Verifica no Profile Inspector.",
            target.export_name
        )));
    }
    status(app)
}

fn save_snapshot(target: &Target, path: &Path, block: &str) -> Result<()> {
    std::fs::write(path, format!("\u{feff}{}", wrap_profiles(block)))?;
    journal::record(target.tweak_id, Undo::NvidiaBaseProfile { snapshot: path.to_path_buf() })
}

/// Undo step: replace-import a saved profile snapshot.
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
        let b = extract_profile(EXPORT, "Base Profile", "").unwrap();
        assert!(b.contains("<ProfileName />"));
        assert!(b.contains("<SettingID>8102046</SettingID>"));
        assert!(!b.contains("Counter-strike"));
    }

    #[test]
    fn missing_global_is_none() {
        assert!(extract_profile("<ArrayOfProfile></ArrayOfProfile>", "Base Profile", "").is_none());
    }

    #[test]
    fn cs2_profile_keeps_name_and_apps() {
        let b = extract_profile(EXPORT, "counter-strike 2", "Counter-Strike 2").unwrap();
        assert!(b.contains("<ProfileName>Counter-Strike 2</ProfileName>"));
        let o = NvCs2Options { low_latency: true, max_performance_power: false, vsync_off: true, texture_high_performance: false };
        let nip = render_nip(&CS2, &settings_for_cs2(&o));
        assert!(nip.contains("<Executeables><string>cs2.exe</string></Executeables>"));
        assert!(nip.contains(&format!("<SettingValue>{}</SettingValue>", VSYNCMODE_FORCEOFF)));
    }

    #[test]
    fn strip_settings_keeps_apps() {
        let block = "<Profile><ProfileName>X</ProfileName><Executeables><string>a.exe</string></Executeables><Settings><ProfileSetting /></Settings></Profile>";
        let out = strip_settings(block);
        assert!(out.contains("<string>a.exe</string>"));
        assert!(out.contains("<Settings />"));
        assert!(!out.contains("<ProfileSetting"));
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
        let nip = render_nip(&GLOBAL, &s);
        assert!(nip.contains("<SettingID>8102046</SettingID>")); // PRERENDERLIMIT 0x007BA09E
        assert!(nip.contains("<SettingValue>4294967295</SettingValue>"));
        assert!(profile_contains(&nip, &s));
        assert!(!profile_contains(EXPORT, &s));
    }
}
