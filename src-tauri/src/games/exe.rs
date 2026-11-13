//! Picking a game's executable inside its install folder.
//!
//! Launchers only tell us the folder, and the process we must watch (and give
//! priority to) is often not the obvious one: Unreal games start a small
//! bootstrapper that runs `<Name>-Win64-Shipping.exe`. Every `.exe` is scored
//! and the best few are offered; the user can pick another in the UI.

use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 5;
const MAX_ENTRIES: usize = 30_000;
pub const MAX_CANDIDATES: usize = 8;

/// Folders that never hold the game itself (installers, redistributables, anti-cheat).
const SKIP_DIRS: &[&str] = &[
    "_commonredist",
    "commonredist",
    "redist",
    "redistributables",
    "directx",
    "dotnet",
    "vcredist",
    "easyanticheat",
    "battleye",
    "__installer",
    "installers",
    "prereqs",
    "support",
    "crashreporter",
    "crashpad",
    "$recycle.bin",
];

/// File-name fragments of helpers that are never the game process.
const SKIP_FILES: &[&str] = &[
    "unins",
    "setup",
    "install",
    "redist",
    "dxsetup",
    "dotnet",
    "crash",
    "report",
    "easyanticheat",
    "beservice",
    "battleye",
    "prereq",
    "cefprocess",
    "webhelper",
    "helper",
    "updater",
    "uploader",
    "touchup",
    "cleanup",
    "vc_redist",
    "ue4prereq",
    "dxwebsetup",
];

/// Keep only letters and digits, lowercase: "Counter-Strike 2" → "counterstrike2".
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn skipped_file(stem_lower: &str) -> bool {
    SKIP_FILES.iter().any(|p| stem_lower.contains(p))
}

/// Higher is more likely the game. `rel` is the path relative to the install
/// folder, `names` the game's display name and folder name.
pub fn score(rel: &Path, size: u64, names: &[&str]) -> i64 {
    let stem = rel.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let rel_lower = rel.to_string_lossy().to_lowercase();
    let mut s = (size / (1024 * 1024)).min(400) as i64;
    if stem.contains("shipping") {
        s += 1000;
    }
    let ns = norm(&stem);
    if !ns.is_empty() {
        for n in names.iter().map(|n| norm(n)).filter(|n| n.len() >= 2) {
            if ns == n {
                s += 700;
            } else if ns.contains(&n) || n.contains(&ns) {
                s += 500;
            }
        }
    }
    if rel_lower.contains("win64") || rel_lower.contains("x64") {
        s += 60;
    }
    if stem.contains("launcher") || stem.contains("bootstrap") || stem.contains("start") {
        s -= 400;
    }
    if stem.contains("server") || stem.contains("editor") || stem.contains("benchmark") || stem.contains("config") {
        s -= 300;
    }
    // Deep paths are usually tools; the Unreal Binaries\Win64 depth (4) is fine.
    s -= rel.components().count().saturating_sub(4) as i64 * 30;
    s
}

/// Ranked `.exe` files under `dir`, best first.
pub fn candidates(dir: &Path, names: &[&str]) -> Vec<PathBuf> {
    let mut found: Vec<(i64, PathBuf)> = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    let mut seen = 0usize;
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                break;
            }
            let Ok(ft) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().to_lowercase();
            let path = e.path();
            if ft.is_dir() {
                if depth < MAX_DEPTH && !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push((path, depth + 1));
                }
            } else if ft.is_file() && name.ends_with(".exe") {
                let stem = name.trim_end_matches(".exe");
                if skipped_file(stem) {
                    continue;
                }
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                let rel = path.strip_prefix(dir).unwrap_or(&path).to_path_buf();
                found.push((score(&rel, size, names), path));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().take(MAX_CANDIDATES).map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    fn best(files: &[(&str, u64)], names: &[&str]) -> String {
        files
            .iter()
            .max_by_key(|(p, size)| score(Path::new(p), *size, names))
            .map(|(p, _)| p.to_string())
            .unwrap()
    }

    #[test]
    fn unreal_shipping_beats_bootstrapper() {
        let files = [
            ("Palworld.exe", MB / 2),
            ("Pal/Binaries/Win64/Palworld-Win64-Shipping.exe", 140 * MB),
        ];
        assert_eq!(best(&files, &["Palworld", "Palworld"]), "Pal/Binaries/Win64/Palworld-Win64-Shipping.exe");
    }

    #[test]
    fn name_match_beats_bigger_tool() {
        let files = [("bin/x64/eldenring.exe", 80 * MB), ("tools/ModEditor.exe", 300 * MB)];
        assert_eq!(best(&files, &["ELDEN RING", "ELDEN RING"]), "bin/x64/eldenring.exe");
    }

    #[test]
    fn launcher_is_ranked_down() {
        let files = [("GameLauncher.exe", 30 * MB), ("Game.exe", 25 * MB)];
        assert_eq!(best(&files, &["Game", "Game"]), "Game.exe");
    }

    #[test]
    fn helpers_are_skipped() {
        assert!(skipped_file("unins000"));
        assert!(skipped_file("unitycrashhandler64"));
        assert!(skipped_file("vc_redist.x64"));
        assert!(!skipped_file("cs2"));
    }

    #[test]
    fn scans_a_folder() {
        let root = std::env::temp_dir().join(format!("vektor-exe-test-{}", std::process::id()));
        let bin = root.join("Game").join("Binaries").join("Win64");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(root.join("_CommonRedist")).unwrap();
        std::fs::write(root.join("Game.exe"), vec![0u8; 1024]).unwrap();
        std::fs::write(bin.join("Game-Win64-Shipping.exe"), vec![0u8; 4096]).unwrap();
        std::fs::write(root.join("unins000.exe"), b"x").unwrap();
        std::fs::write(root.join("_CommonRedist").join("vcredist.exe"), b"x").unwrap();
        let c = candidates(&root, &["Game"]);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(c.len(), 2);
        assert!(c[0].ends_with("Game-Win64-Shipping.exe"));
    }
}
