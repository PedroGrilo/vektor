//! Registry access (64-bit view) plus journaled writes.

use crate::error::Result;
use crate::journal::{self, Hive, RegValue, Undo};

/// Read a value; `Ok(None)` when the key or value does not exist.
pub fn read(hive: Hive, key: &str, name: &str) -> Result<Option<RegValue>> {
    imp::read(hive, key, name)
}

/// Write a value, creating the key if needed.
pub fn write(hive: Hive, key: &str, name: &str, value: &RegValue) -> Result<()> {
    imp::write(hive, key, name, value)
}

/// Delete a value. Missing key/value is not an error.
pub fn delete(hive: Hive, key: &str, name: &str) -> Result<()> {
    imp::delete(hive, key, name)
}

/// Names of the direct subkeys of `key` (empty when the key is missing).
pub fn subkeys(hive: Hive, key: &str) -> Vec<String> {
    imp::subkeys(hive, key)
}

/// Names of the values directly under `key` (empty when the key is missing).
pub fn value_names(hive: Hive, key: &str) -> Vec<String> {
    imp::value_names(hive, key)
}

/// Snapshot the current value into the journal under `tweak_id`, then write.
pub fn set_journaled(tweak_id: &str, hive: Hive, key: &str, name: &str, value: &RegValue) -> Result<()> {
    let previous = read(hive, key, name)?;
    journal::record(
        tweak_id,
        Undo::RegistryValue { hive, key: key.to_string(), name: name.to_string(), previous },
    )?;
    write(hive, key, name, value)
}

#[cfg(windows)]
mod imp {
    use std::io::ErrorKind;

    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_64KEY, KEY_WRITE,
        RegType,
    };
    use winreg::RegKey;

    use crate::error::{Error, Result};
    use crate::journal::{Hive, RegValue};

    fn root(hive: Hive) -> RegKey {
        RegKey::predef(match hive {
            Hive::Hkcu => HKEY_CURRENT_USER,
            Hive::Hklm => HKEY_LOCAL_MACHINE,
        })
    }

    fn ctx(e: std::io::Error, key: &str, name: &str) -> Error {
        Error::msg(format!("Registo {key}\\{name}: {e}"))
    }

    pub fn read(hive: Hive, key: &str, name: &str) -> Result<Option<RegValue>> {
        let k = match root(hive).open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY) {
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ctx(e, key, name)),
        };
        let raw = match k.get_raw_value(name) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ctx(e, key, name)),
        };
        let b = &raw.bytes;
        let v = match raw.vtype {
            RegType::REG_DWORD if b.len() >= 4 => RegValue::Dword(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            RegType::REG_QWORD if b.len() >= 8 => {
                let mut a = [0u8; 8];
                a.copy_from_slice(&b[..8]);
                RegValue::Qword(u64::from_le_bytes(a))
            }
            RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
                RegValue::Sz(k.get_value::<String, _>(name).map_err(|e| ctx(e, key, name))?)
            }
            RegType::REG_MULTI_SZ => {
                RegValue::MultiSz(k.get_value::<Vec<String>, _>(name).map_err(|e| ctx(e, key, name))?)
            }
            RegType::REG_BINARY => RegValue::Binary(b.to_vec()),
            // Vektor only snapshots types it can restore exactly.
            other => {
                return Err(Error::msg(format!(
                    "Registo {key}\\{name}: tipo {other:?} não suportado para backup"
                )))
            }
        };
        Ok(Some(v))
    }

    pub fn write(hive: Hive, key: &str, name: &str, value: &RegValue) -> Result<()> {
        let (k, _) = root(hive)
            .create_subkey_with_flags(key, KEY_WRITE | KEY_WOW64_64KEY)
            .map_err(|e| ctx(e, key, name))?;
        let r = match value {
            RegValue::Dword(v) => k.set_value(name, v),
            RegValue::Qword(v) => k.set_value(name, v),
            RegValue::Sz(v) => k.set_value(name, v),
            RegValue::MultiSz(v) => k.set_value(name, v),
            RegValue::Binary(v) => k.set_raw_value(
                name,
                &winreg::RegValue { bytes: v.clone().into(), vtype: RegType::REG_BINARY },
            ),
        };
        r.map_err(|e| ctx(e, key, name))
    }

    pub fn delete(hive: Hive, key: &str, name: &str) -> Result<()> {
        let k = match root(hive).open_subkey_with_flags(key, KEY_SET_VALUE | KEY_WOW64_64KEY) {
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(ctx(e, key, name)),
        };
        match k.delete_value(name) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ctx(e, key, name)),
        }
    }

    pub fn value_names(hive: Hive, key: &str) -> Vec<String> {
        root(hive)
            .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY)
            .map(|k| k.enum_values().filter_map(|r| r.ok()).map(|(n, _)| n).collect())
            .unwrap_or_default()
    }

    pub fn subkeys(hive: Hive, key: &str) -> Vec<String> {
        root(hive)
            .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY)
            .map(|k| k.enum_keys().filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::error::{Error, Result};
    use crate::journal::{Hive, RegValue};

    pub fn read(_: Hive, _: &str, _: &str) -> Result<Option<RegValue>> {
        Err(Error::Unsupported)
    }
    pub fn write(_: Hive, _: &str, _: &str, _: &RegValue) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub fn delete(_: Hive, _: &str, _: &str) -> Result<()> {
        Err(Error::Unsupported)
    }
    pub fn subkeys(_: Hive, _: &str) -> Vec<String> {
        Vec::new()
    }
    pub fn value_names(_: Hive, _: &str) -> Vec<String> {
        Vec::new()
    }
}
