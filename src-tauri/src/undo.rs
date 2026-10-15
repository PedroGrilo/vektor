//! Executes a single journal [`Undo`] step by delegating to the owning module.

use crate::error::Result;
use crate::journal::Undo;
use crate::{nvidia, win};

pub fn run(step: &Undo) -> Result<()> {
    match step {
        Undo::RegistryValue { hive, key, name, previous } => match previous {
            Some(v) => win::registry::write(*hive, key, name, v),
            None => win::registry::delete(*hive, key, name),
        },
        Undo::PowerPlan { previous_guid, created_guid } => {
            win::power::restore(previous_guid, created_guid.as_deref())
        }
        Undo::Service { name, previous_start, was_running } => {
            win::services::restore(name, *previous_start, *was_running)
        }
        Undo::File { path, backup } => win::files::restore(path, backup.as_deref()),
        Undo::FileReadonly { path, was_readonly } => win::files::set_readonly(path, *was_readonly),
        Undo::NvidiaBaseProfile { snapshot } => nvidia::restore_base_profile(snapshot),
    }
}
