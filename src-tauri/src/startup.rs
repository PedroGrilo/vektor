//! Programs that start with Windows (Run keys and Startup folders).
//!
//! Entries are switched on and off exactly like Task Manager's Startup tab: a
//! flag under `Explorer\StartupApproved`, so the program's own entry is never
//! deleted and Task Manager shows the same state. Every change is journaled.

use std::path::PathBuf;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::journal::{self, Hive, RegValue};
use crate::win::registry;

const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

#[derive(Debug, Clone, Copy)]
enum Kind {
    Registry(&'static str),
    UserFolder,
    CommonFolder,
}

struct Location {
    id: &'static str,
    label: &'static str,
    hive: Hive,
    kind: Kind,
    /// Subkey of StartupApproved holding the flags.
    approved: &'static str,
}

const LOCATIONS: &[Location] = &[
    Location { id: "hkcu-run", label: "Registo (utilizador)", hive: Hive::Hkcu, kind: Kind::Registry(r"Software\Microsoft\Windows\CurrentVersion\Run"), approved: "Run" },
    Location { id: "hklm-run", label: "Registo (todos os utilizadores)", hive: Hive::Hklm, kind: Kind::Registry(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run"), approved: "Run" },
    Location { id: "hklm-run32", label: "Registo (32 bits)", hive: Hive::Hklm, kind: Kind::Registry(r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"), approved: "Run32" },
    Location { id: "user-folder", label: "Pasta Arranque (utilizador)", hive: Hive::Hkcu, kind: Kind::UserFolder, approved: "StartupFolder" },
    Location { id: "common-folder", label: "Pasta Arranque (todos)", hive: Hive::Hklm, kind: Kind::CommonFolder, approved: "StartupFolder" },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntry {
    /// "<location>|<name>"
    pub id: String,
    pub name: String,
    pub command: String,
    pub location: String,
    pub enabled: bool,
    /// Changed by Vektor (can be reverted in Restaurar).
    pub changed_by_vektor: bool,
    /// Vektor will not switch it off (security, Vektor itself).
    pub protected: bool,
    /// What happens if it is switched off, when known.
    pub hint: Option<String>,
}

/// (fragment of name or command, lowercase) → hint
const HINTS: &[(&str, &str)] = &[
    ("securityhealth", "Ícone da Segurança do Windows. Não desligar."),
    ("vektor", "Gerido em Definições → Arrancar com o Windows."),
    ("rtkaud", "Painel de áudio Realtek: efeitos e deteção de auscultadores podem deixar de funcionar."),
    ("realtek", "Software de áudio ou rede Realtek: pode controlar efeitos de som."),
    ("nvidia", "Componente do driver NVIDIA (overlay, atualizações). Normalmente seguro de desligar."),
    ("radeonsoftware", "Software AMD (Adrenalin). Desligado, perdes o overlay e os atalhos até o abrires."),
    ("onedrive", "Os ficheiros deixam de sincronizar até abrires o OneDrive."),
    ("discord", "O Discord deixa de abrir sozinho."),
    ("spotify", "O Spotify deixa de abrir sozinho."),
    ("steam", "A Steam deixa de abrir sozinha; abre-a antes de jogar."),
    ("epicgameslauncher", "A Epic deixa de abrir sozinha."),
    ("teams", "O Teams deixa de abrir sozinho."),
    ("msedge", "O Edge deixa de pré-carregar no arranque."),
    ("lghub", "Software Logitech: perfis de rato e teclado só funcionam com ele aberto."),
    ("razer", "Software Razer: perfis e iluminação só funcionam com ele aberto."),
    ("icue", "Corsair iCUE: perfis e iluminação só funcionam com ele aberto."),
];

fn hint_for(name: &str, command: &str) -> Option<String> {
    let hay = format!("{} {}", name.to_lowercase(), command.to_lowercase());
    HINTS.iter().find(|(k, _)| hay.contains(k)).map(|(_, h)| h.to_string())
}

fn is_protected(name: &str, command: &str) -> bool {
    let hay = format!("{} {}", name.to_lowercase(), command.to_lowercase());
    hay.contains("securityhealth") || hay.contains("vektor")
}

/// Task Manager's rule: the first byte is even when the entry is enabled.
pub fn bytes_enabled(b: &[u8]) -> bool {
    b.first().is_none_or(|x| x & 1 == 0)
}

/// 12-byte StartupApproved value: flag + FILETIME of the change.
pub fn approved_bytes(enabled: bool, filetime: u64) -> Vec<u8> {
    let mut v = vec![if enabled { 2 } else { 3 }, 0, 0, 0];
    v.extend_from_slice(&if enabled { 0 } else { filetime }.to_le_bytes());
    v
}

fn filetime_now() -> u64 {
    // 100 ns intervals since 1601-01-01.
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    (d.as_secs() + EPOCH_DIFF_SECS) * 10_000_000 + u64::from(d.subsec_nanos() / 100)
}

fn folder(kind: Kind) -> Option<PathBuf> {
    let (var, rest) = match kind {
        Kind::UserFolder => ("APPDATA", r"Microsoft\Windows\Start Menu\Programs\Startup"),
        Kind::CommonFolder => ("ProgramData", r"Microsoft\Windows\Start Menu\Programs\StartUp"),
        Kind::Registry(_) => return None,
    };
    Some(PathBuf::from(std::env::var_os(var)?).join(rest))
}

/// (name, command) pairs at a location.
fn raw_entries(loc: &Location) -> Vec<(String, String)> {
    match loc.kind {
        Kind::Registry(key) => registry::value_names(loc.hive, key)
            .into_iter()
            .filter(|n| !n.is_empty())
            .filter_map(|n| match registry::read(loc.hive, key, &n) {
                Ok(Some(RegValue::Sz(cmd))) => Some((n, cmd)),
                _ => None,
            })
            .collect(),
        Kind::UserFolder | Kind::CommonFolder => {
            let Some(dir) = folder(loc.kind) else { return Vec::new() };
            let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
            rd.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| !n.eq_ignore_ascii_case("desktop.ini"))
                .map(|n| (n.clone(), dir.join(&n).to_string_lossy().to_string()))
                .collect()
        }
    }
}

fn approved_key(loc: &Location) -> String {
    format!(r"{APPROVED}\{}", loc.approved)
}

fn journal_id(loc: &Location, name: &str) -> String {
    format!("startup:{}|{name}", loc.id)
}

fn enabled_at(loc: &Location, name: &str) -> bool {
    match registry::read(loc.hive, &approved_key(loc), name) {
        Ok(Some(RegValue::Binary(b))) => bytes_enabled(&b),
        _ => true,
    }
}

pub fn list() -> Result<Vec<StartupEntry>> {
    let applied = journal::applied_ids();
    let mut out = Vec::new();
    for loc in LOCATIONS {
        for (name, command) in raw_entries(loc) {
            out.push(StartupEntry {
                id: format!("{}|{name}", loc.id),
                enabled: enabled_at(loc, &name),
                changed_by_vektor: applied.contains(&journal_id(loc, &name)),
                protected: is_protected(&name, &command),
                hint: hint_for(&name, &command),
                location: loc.label.to_string(),
                name,
                command,
            });
        }
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    Ok(out)
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<Vec<StartupEntry>> {
    let (loc_id, name) = id.split_once('|').ok_or_else(|| Error::InvalidInput(format!("entrada inválida: {id}")))?;
    let loc = LOCATIONS.iter().find(|l| l.id == loc_id).ok_or_else(|| Error::InvalidInput(format!("local desconhecido: {loc_id}")))?;
    let (_, command) = raw_entries(loc)
        .into_iter()
        .find(|(n, _)| n == name)
        .ok_or_else(|| Error::InvalidInput(format!("entrada não encontrada: {name}")))?;
    if !enabled && is_protected(name, &command) {
        return Err(Error::InvalidInput(format!("{name} não pode ser desligado aqui")));
    }
    let jid = journal_id(loc, name);
    if enabled_at(loc, name) != enabled {
        if enabled && journal::is_applied(&jid) {
            // Undo our own change: puts back exactly what was there before.
            journal::revert(&jid)?;
        } else {
            let value = RegValue::Binary(approved_bytes(enabled, filetime_now()));
            registry::set_journaled(&jid, loc.hive, &approved_key(loc), name, &value)?;
        }
    }
    list()
}

/// Friendly name for a `startup:` journal id, for the Restore page.
pub fn describe_journal_id(jid: &str) -> Option<String> {
    let (_, name) = jid.strip_prefix("startup:")?.split_once('|')?;
    Some(format!("Arranque: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_parity() {
        assert!(bytes_enabled(&[2, 0, 0, 0]));
        assert!(bytes_enabled(&[6, 0]));
        assert!(!bytes_enabled(&[3, 0, 0, 0]));
        assert!(!bytes_enabled(&[7]));
        assert!(bytes_enabled(&[]));
    }

    #[test]
    fn approved_value_layout() {
        let off = approved_bytes(false, 0x0102_0304_0506_0708);
        assert_eq!(off.len(), 12);
        assert_eq!(off[0], 3);
        assert_eq!(&off[4..], &0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(approved_bytes(true, 99), vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn protects_security_and_vektor() {
        assert!(is_protected("SecurityHealth", r"%windir%\system32\SecurityHealthSystray.exe"));
        assert!(is_protected("Vektor", r"C:\Program Files\Vektor\vektor.exe"));
        assert!(!is_protected("Discord", r"C:\Users\x\Discord\Update.exe"));
    }

    #[test]
    fn journal_ids_describe() {
        assert_eq!(describe_journal_id("startup:hkcu-run|Discord").as_deref(), Some("Arranque: Discord"));
    }
}
