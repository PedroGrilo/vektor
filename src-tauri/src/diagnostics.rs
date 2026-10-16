//! Support bundle: a zip on the Desktop with system info, applied tweaks, the
//! rollback journal and Vektor's own state files. Steam identities are redacted.

use std::io::Write;
use std::path::PathBuf;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::{Error, Result};
use crate::journal;

/// State files copied verbatim when present.
const STATE_FILES: &[&str] = &[
    "journal.json",
    "memory.json",
    "memory-status.json",
    "session.json",
    "settings.json",
    "benchmarks.json",
];

#[cfg(windows)]
fn desktop_dir() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    // SAFETY: SHGetKnownFolderPath returns a CoTaskMemAlloc'd PWSTR we free below.
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.map(PathBuf::from)
    }
}

#[cfg(not(windows))]
fn desktop_dir() -> Option<PathBuf> {
    None
}

/// CS2 detection without Steam account identities.
fn redacted_cs2() -> serde_json::Value {
    match crate::cs2::detect() {
        Ok(c) => serde_json::json!({
            "steamPath": c.steam_path,
            "gamePath": c.game_path,
            "installed": c.installed,
            "running": c.running,
            "userCount": c.users.len(),
            "launchOptions": c.launch_options,
            "updateRequired": c.update_required,
            "buildId": c.build_id,
        }),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    }
}

fn to_json<T: serde::Serialize>(r: Result<T>) -> Vec<u8> {
    let v = match r {
        Ok(v) => serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };
    serde_json::to_vec_pretty(&v).unwrap_or_default()
}

fn file_name(timestamp: &str) -> String {
    let safe: String = timestamp.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    format!("Vektor-diagnostico-{safe}.zip")
}

/// Writes a zip to the Desktop and returns its path.
pub fn export() -> Result<String> {
    let dir = desktop_dir().unwrap_or_else(journal::data_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(file_name(&crate::memory::now_iso8601()));

    let zip_err = |e: zip::result::ZipError| Error::msg(format!("Erro ao criar o zip: {e}"));
    let mut zip = ZipWriter::new(std::fs::File::create(&path)?);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut add = |name: &str, data: &[u8]| -> Result<()> {
        zip.start_file(name, opts).map_err(zip_err)?;
        zip.write_all(data)?;
        Ok(())
    };

    add("version.txt", env!("CARGO_PKG_VERSION").as_bytes())?;
    add("system.json", &to_json(crate::system::info()))?;
    add("tweaks.json", &to_json(Ok(crate::tweaks::list())))?;
    add("cs2.json", &serde_json::to_vec_pretty(&redacted_cs2())?)?;
    for f in STATE_FILES {
        if let Ok(bytes) = std::fs::read(journal::data_dir().join(f)) {
            add(f, &bytes)?;
        }
    }
    zip.finish().map_err(zip_err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_safe() {
        assert_eq!(file_name("2026-10-16T09:30:00Z"), "Vektor-diagnostico-2026-10-16T09-30-00Z.zip");
    }
}
