//! File backup/restore helpers used by the journal.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;
use crate::journal::{self, Undo};

/// Restore `path` from `backup`, or delete `path` when `backup` is None
/// (meaning Vektor created the file).
pub fn restore(path: &Path, backup: Option<&Path>) -> Result<()> {
    match backup {
        Some(b) => {
            if path.exists() {
                set_readonly(path, false)?;
            }
            std::fs::copy(b, path)?;
            // Backup is no longer needed once restored.
            let _ = std::fs::remove_file(b);
            Ok(())
        }
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                set_readonly(path, false)?;
                std::fs::remove_file(path)?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        },
    }
}

pub fn set_readonly(path: &Path, readonly: bool) -> Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    if perms.readonly() != readonly {
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Back up `path` (if it exists) into the journal backup dir and record an
/// `Undo::File` step under `tweak_id`. Call before overwriting or creating `path`.
#[allow(dead_code)] // shared helper for modules that rewrite user files
pub fn backup_journaled(tweak_id: &str, path: &Path) -> Result<()> {
    let backup = if path.exists() {
        let dir = journal::backup_dir();
        std::fs::create_dir_all(&dir)?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let b: PathBuf = dir.join(format!("{stamp}-{file}.bak"));
        std::fs::copy(path, &b)?;
        Some(b)
    } else {
        None
    };
    journal::record(tweak_id, Undo::File { path: path.to_path_buf(), backup })
}
