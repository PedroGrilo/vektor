//! Shader cache cleaner. Drivers and Steam rebuild these caches on demand;
//! clearing them fixes stutter after driver updates (first matches may
//! stutter briefly while shaders recompile).

use std::path::{Path, PathBuf};

use super::CleanReport;
use crate::error::{Error, Result};

/// Cache directories relative to %LOCALAPPDATA%.
const LOCAL_CACHES: &[&str] = &[
    "D3DSCache",
    r"NVIDIA\DXCache",
    r"NVIDIA\GLCache",
    r"AMD\DxCache",
    r"AMD\DxcCache",
    r"AMD\VkCache",
    r"Intel\ShaderCache",
];

fn cache_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        dirs.extend(LOCAL_CACHES.iter().map(|c| local.join(c)));
    }
    if let Some(steam) = crate::cs2::steam_path() {
        dirs.push(steam.join("steamapps").join("shadercache").join("730"));
    }
    dirs
}

pub fn clean_shader_caches() -> Result<CleanReport> {
    if !cfg!(windows) {
        return Err(Error::Unsupported);
    }
    let mut report = CleanReport::default();
    for dir in cache_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let before = report.freed_bytes;
        clean_dir(&dir, &mut report);
        if report.freed_bytes > before {
            report.paths.push(dir.to_string_lossy().into_owned());
        }
    }
    Ok(report)
}

/// Delete the contents of `dir` (not the dir itself). Files in use are skipped.
fn clean_dir(dir: &Path, report: &mut CleanReport) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        // Never follow symlinks/junctions out of the cache directory.
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            clean_dir(&path, report);
            let _ = std::fs::remove_dir(&path);
        } else if std::fs::remove_file(&path).is_ok() {
            report.freed_bytes += meta.len();
            report.files_removed += 1;
        }
    }
}
