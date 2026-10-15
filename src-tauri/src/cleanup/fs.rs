//! Filesystem walking shared by scan (measure only) and clean (delete).
//! Only the *contents* of a root are touched; the root itself is never removed,
//! symlinks/junctions are never followed and files in use are skipped.

use std::path::Path;
use std::time::{Duration, SystemTime};

/// Which files under a root qualify for cleaning.
#[derive(Clone, Copy)]
pub struct Filter {
    /// Skip files modified more recently than this (protects running installers etc.).
    pub min_age: Option<Duration>,
    /// Only files whose (lowercase) name matches; `None` = every file.
    pub name: Option<fn(&str) -> bool>,
    /// Descend into subdirectories.
    pub recursive: bool,
}

impl Filter {
    pub const ALL: Filter = Filter { min_age: None, name: None, recursive: true };
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub bytes: u64,
    pub files: u64,
}

/// True when a file last modified at `modified` is old enough to delete.
pub fn old_enough(modified: Option<SystemTime>, min_age: Option<Duration>, now: SystemTime) -> bool {
    let Some(min_age) = min_age else { return true };
    // Unknown or future timestamps are treated as "recent" (skip).
    match modified.and_then(|m| now.duration_since(m).ok()) {
        Some(age) => age >= min_age,
        None => false,
    }
}

fn name_ok(path: &Path, filter: &Filter) -> bool {
    match filter.name {
        None => true,
        Some(f) => path
            .file_name()
            .map(|n| f(&n.to_string_lossy().to_lowercase()))
            .unwrap_or(false),
    }
}

#[cfg(windows)]
fn is_link(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_type().is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

/// Walk the contents of `root`. With `delete == false` only measures.
pub fn walk(root: &Path, filter: &Filter, delete: bool) -> Stats {
    let mut stats = Stats::default();
    let now = SystemTime::now();
    walk_dir(root, filter, delete, now, &mut stats);
    stats
}

fn walk_dir(dir: &Path, filter: &Filter, delete: bool, now: SystemTime, stats: &mut Stats) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if is_link(&meta) {
            continue;
        }
        if meta.is_dir() {
            if filter.recursive {
                walk_dir(&path, filter, delete, now, stats);
                if delete {
                    // Succeeds only when everything inside was removed.
                    let _ = std::fs::remove_dir(&path);
                }
            }
            continue;
        }
        if !name_ok(&path, filter) || !old_enough(meta.modified().ok(), filter.min_age, now) {
            continue;
        }
        if !delete || remove_file(&path, &meta) {
            stats.bytes += meta.len();
            stats.files += 1;
        }
    }
}

/// Measure or delete a single file (e.g. MEMORY.DMP).
pub fn single_file(path: &Path, delete: bool) -> Stats {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return Stats::default() };
    if !meta.is_file() || is_link(&meta) {
        return Stats::default();
    }
    if !delete || remove_file(path, &meta) {
        Stats { bytes: meta.len(), files: 1 }
    } else {
        Stats::default()
    }
}

fn remove_file(path: &Path, meta: &std::fs::Metadata) -> bool {
    if std::fs::remove_file(path).is_ok() {
        return true;
    }
    // Read-only files (common in Temp) fail on Windows: clear the flag and retry.
    if meta.permissions().readonly() {
        let mut p = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        if std::fs::set_permissions(path, p).is_ok() {
            return std::fs::remove_file(path).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: Duration = Duration::from_secs(24 * 3600);

    #[test]
    fn age_filter() {
        let now = SystemTime::now();
        assert!(old_enough(Some(now - 2 * DAY), Some(DAY), now));
        assert!(!old_enough(Some(now - Duration::from_secs(60)), Some(DAY), now));
        assert!(!old_enough(None, Some(DAY), now), "unknown mtime must be skipped");
        assert!(!old_enough(Some(now + DAY), Some(DAY), now), "future mtime must be skipped");
        assert!(old_enough(None, None, now));
    }

    #[test]
    fn walk_keeps_root_and_respects_filter() {
        let root = std::env::temp_dir().join(format!("vektor-clean-test-{}", std::process::id()));
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("a.pf"), b"12345").unwrap();
        std::fs::write(root.join("keep.txt"), b"xx").unwrap();
        std::fs::write(sub.join("b.pf"), b"123").unwrap();

        let pf = Filter { min_age: None, name: Some(|n| n.ends_with(".pf")), recursive: false };
        assert_eq!(walk(&root, &pf, false), Stats { bytes: 5, files: 1 });

        let all = walk(&root, &Filter::ALL, false);
        assert_eq!(all, Stats { bytes: 10, files: 3 });

        let deleted = walk(&root, &Filter::ALL, true);
        assert_eq!(deleted.files, 3);
        assert!(root.is_dir(), "root must never be removed");
        assert!(!sub.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
