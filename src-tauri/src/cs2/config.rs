//! Reads and writes Vektor's CS2 config:
//! - `<game>/game/csgo/cfg/vektor.cfg` (convars, binds, reload alias)
//! - `exec vektor` line in `cfg/autoexec.cfg`
//! - merged values in `userdata/<acct>/730/local/cfg/cs2_video.txt`
//!
//! Every file is backed up and journaled under [`TWEAK_ID`] before the first write.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::vdf::{self, Vdf};
use super::{Cs2Config, VideoSettings};
use crate::error::{Error, Result};
use crate::journal::{self, Undo};

pub const TWEAK_ID: &str = "cs2-config";
pub const CFG_NAME: &str = "vektor";
const RELOAD_ALIAS: &str = "vektor_reload";

// ---------- validation ----------

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | '-'))
}

fn valid_value(s: &str) -> bool {
    s.len() <= 256 && !s.chars().any(|c| matches!(c, '"' | ';' | '\n' | '\r' | '\0'))
}

pub fn validate(cfg: &Cs2Config) -> Result<()> {
    for (k, v) in &cfg.convars {
        if !valid_name(k) {
            return Err(Error::InvalidInput(format!("nome de convar inválido: {k}")));
        }
        if !valid_value(v) {
            return Err(Error::InvalidInput(format!("valor inválido para {k}")));
        }
    }
    for (k, v) in &cfg.binds {
        if !valid_name(k) {
            return Err(Error::InvalidInput(format!("tecla inválida: {k}")));
        }
        if v.is_empty() || !valid_value(v) {
            return Err(Error::InvalidInput(format!("comando inválido para a tecla {k}")));
        }
    }
    if let Some(k) = cfg.reload_key.as_ref().filter(|k| !valid_name(k)) {
        return Err(Error::InvalidInput(format!("tecla de recarregar inválida: {k}")));
    }
    for (k, v) in &cfg.video.values {
        if !valid_name(k) || !valid_value(v) {
            return Err(Error::InvalidInput(format!("definição de vídeo inválida: {k}")));
        }
    }
    Ok(())
}

// ---------- rendering / parsing vektor.cfg ----------

/// Values that are plain numbers/words stay bare; anything else is quoted.
fn cfg_value(v: &str) -> String {
    if !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) {
        v.to_string()
    } else {
        format!("\"{v}\"")
    }
}

pub fn render_cfg(cfg: &Cs2Config) -> String {
    let mut out = String::from("// Gerado pelo Vektor — alterações manuais serão substituídas.\n\n");
    if !cfg.convars.is_empty() {
        out.push_str("// Convars\n");
        for (k, v) in &cfg.convars {
            out.push_str(&format!("{k} {}\n", cfg_value(v)));
        }
        out.push('\n');
    }
    if !cfg.binds.is_empty() {
        out.push_str("// Binds\n");
        for (k, v) in &cfg.binds {
            out.push_str(&format!("bind \"{k}\" \"{v}\"\n"));
        }
        out.push('\n');
    }
    out.push_str(&format!("alias {RELOAD_ALIAS} \"exec {CFG_NAME}\"\n"));
    if let Some(k) = &cfg.reload_key {
        out.push_str(&format!("bind \"{k}\" \"{RELOAD_ALIAS}\"\n"));
    }
    out.push_str("echo \"[Vektor] config carregada\"\n");
    out
}

/// Split a cfg line into tokens, honouring double quotes.
fn split_tokens(line: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut had = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                had = true;
            }
            c if c.is_whitespace() && !quoted => {
                if had || !cur.is_empty() {
                    toks.push(std::mem::take(&mut cur));
                    had = false;
                }
            }
            c => cur.push(c),
        }
    }
    if had || !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

pub fn parse_cfg(text: &str) -> Cs2Config {
    let mut cfg = Cs2Config::default();
    for raw in text.lines() {
        let line = raw.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let t = split_tokens(line);
        match t.first().map(String::as_str) {
            Some("alias") | Some("echo") | Some("exec") => {}
            Some("bind") if t.len() >= 3 => {
                if t[2] == RELOAD_ALIAS {
                    cfg.reload_key = Some(t[1].clone());
                } else {
                    cfg.binds.insert(t[1].clone(), t[2].clone());
                }
            }
            Some(name) if t.len() >= 2 && valid_name(name) => {
                cfg.convars.insert(name.to_string(), t[1..].join(" "));
            }
            _ => {}
        }
    }
    cfg
}

/// Return autoexec text that contains `exec vektor`, or None if already present.
pub fn ensure_exec_line(existing: &str) -> Option<String> {
    let present = existing.lines().any(|l| {
        let t = split_tokens(l.split("//").next().unwrap_or(""));
        t.len() >= 2 && t[0] == "exec" && t[1].trim_end_matches(".cfg").eq_ignore_ascii_case(CFG_NAME)
    });
    if present {
        return None;
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("exec {CFG_NAME}\n"));
    Some(out)
}

// ---------- cs2_video.txt ----------

/// Flatten the video settings into the key/value overrides written to the file.
pub fn video_overrides(v: &VideoSettings) -> BTreeMap<String, String> {
    let mut m = v.values.clone();
    if let Some(w) = v.width {
        m.insert("setting.defaultres".into(), w.to_string());
    }
    if let Some(h) = v.height {
        m.insert("setting.defaultresheight".into(), h.to_string());
    }
    if let Some(r) = v.refresh_rate {
        m.insert("setting.refreshrate_numerator".into(), r.to_string());
        m.insert("setting.refreshrate_denominator".into(), "1".into());
    }
    m
}

/// Merge overrides into an existing video file, preserving order and root name.
pub fn merge_video(existing: Option<&str>, overrides: &BTreeMap<String, String>) -> Result<String> {
    let (root_name, mut pairs): (String, Vec<(String, String)>) = match existing {
        Some(text) => {
            let doc = vdf::parse(text).map_err(|e| Error::msg(format!("cs2_video.txt inválido: {e}")))?;
            match doc.entries().first() {
                Some((name, Vdf::Obj(items))) => (
                    name.clone(),
                    items
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect(),
                ),
                _ => ("video.cfg".into(), Vec::new()),
            }
        }
        None => ("video.cfg".into(), Vec::new()),
    };
    for (k, v) in overrides {
        match pairs.iter_mut().find(|(pk, _)| pk.eq_ignore_ascii_case(k)) {
            Some(p) => p.1 = v.clone(),
            None => pairs.push((k.clone(), v.clone())),
        }
    }
    let mut out = format!("{}\n{{\n", vdf::quote(&root_name));
    for (k, v) in pairs {
        out.push_str(&format!("\t{}\t\t{}\n", vdf::quote(&k), vdf::quote(&v)));
    }
    out.push_str("}\n");
    Ok(out)
}

fn read_video(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| vdf::parse(&t).ok())
        .and_then(|d| d.entries().first().map(|(_, v)| v.string_map()))
        .unwrap_or_default()
}

// ---------- paths ----------

fn cfg_dir() -> Result<PathBuf> {
    Ok(super::csgo_dir()?.join("cfg"))
}

fn video_path(account_id: u32) -> Result<PathBuf> {
    Ok(super::userdata_dir(account_id)?
        .join(super::APP_ID)
        .join("local")
        .join("cfg")
        .join("cs2_video.txt"))
}

// ---------- backup + journal ----------

fn backup_root() -> PathBuf {
    journal::backup_dir().join(TWEAK_ID)
}

fn index_path() -> PathBuf {
    backup_root().join("index.json")
}

/// Paths already journaled in the current application of [`TWEAK_ID`].
fn recorded_paths() -> Vec<PathBuf> {
    if !journal::is_applied(TWEAK_ID) {
        return Vec::new();
    }
    std::fs::read(index_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Back up `path` (if it exists) and record the undo step, once per application.
fn journal_file(path: &Path) -> Result<()> {
    let mut recorded = recorded_paths();
    if recorded.iter().any(|p| p == path) {
        return Ok(());
    }
    std::fs::create_dir_all(backup_root())?;
    let backup = if path.is_file() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let dst = backup_root().join(format!("{stamp}-{}-{name}", recorded.len()));
        std::fs::copy(path, &dst)?;
        Some(dst)
    } else {
        None
    };
    journal::record(TWEAK_ID, Undo::File { path: path.to_path_buf(), backup })?;
    recorded.push(path.to_path_buf());
    std::fs::write(index_path(), serde_json::to_vec(&recorded)?)?;
    Ok(())
}

fn write_journaled(path: &Path, contents: &str) -> Result<()> {
    journal_file(path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

// ---------- public API ----------

pub fn read(account_id: u32) -> Result<Cs2Config> {
    let mut cfg = std::fs::read_to_string(cfg_dir()?.join(format!("{CFG_NAME}.cfg")))
        .map(|t| parse_cfg(&t))
        .unwrap_or_default();
    let mut values = read_video(&video_path(account_id)?);
    let num = |m: &mut BTreeMap<String, String>, k: &str| m.remove(k).and_then(|v| v.trim().parse().ok());
    cfg.video.width = num(&mut values, "setting.defaultres");
    cfg.video.height = num(&mut values, "setting.defaultresheight");
    cfg.video.refresh_rate = num(&mut values, "setting.refreshrate_numerator");
    values.remove("setting.refreshrate_denominator");
    cfg.video.values = values;
    Ok(cfg)
}

pub fn apply(account_id: u32, cfg: &Cs2Config) -> Result<()> {
    validate(cfg)?;
    let cfg_dir = cfg_dir()?;
    let overrides = video_overrides(&cfg.video);
    let video = video_path(account_id)?;
    if !overrides.is_empty() && super::is_running() {
        return Err(Error::msg(
            "Fecha o CS2 antes de aplicar as definições de vídeo (o jogo reescreve o ficheiro ao sair).",
        ));
    }

    write_journaled(&cfg_dir.join(format!("{CFG_NAME}.cfg")), &render_cfg(cfg))?;

    let autoexec = cfg_dir.join("autoexec.cfg");
    let existing = std::fs::read_to_string(&autoexec).unwrap_or_default();
    if let Some(text) = ensure_exec_line(&existing) {
        write_journaled(&autoexec, &text)?;
    }

    if !overrides.is_empty() {
        let existing = std::fs::read_to_string(&video).ok();
        let merged = merge_video(existing.as_deref(), &overrides)?;
        write_journaled(&video, &merged)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Cs2Config {
        let mut c = Cs2Config::default();
        c.convars.insert("fps_max".into(), "0".into());
        c.convars.insert("cl_crosshaircolor_r".into(), "0".into());
        c.binds.insert("mwheeldown".into(), "+jump".into());
        c.reload_key = Some("F8".into());
        c
    }

    #[test]
    fn render_and_parse_roundtrip() {
        let c = sample();
        let text = render_cfg(&c);
        assert!(text.contains("alias vektor_reload \"exec vektor\""));
        assert!(text.contains("bind \"F8\" \"vektor_reload\""));
        let back = parse_cfg(&text);
        assert_eq!(back.convars, c.convars);
        assert_eq!(back.binds, c.binds);
        assert_eq!(back.reload_key, c.reload_key);
    }

    #[test]
    fn rejects_injection() {
        let mut c = sample();
        c.convars.insert("fps_max".into(), "0; quit".into());
        assert!(validate(&c).is_err());
        let mut c = sample();
        c.binds.insert("k".into(), "say \"hi\"".into());
        assert!(validate(&c).is_err());
        let mut c = sample();
        c.convars.insert("bad name".into(), "1".into());
        assert!(validate(&c).is_err());
        assert!(validate(&sample()).is_ok());
    }

    #[test]
    fn autoexec_line() {
        assert_eq!(ensure_exec_line("").as_deref(), Some("exec vektor\n"));
        assert_eq!(ensure_exec_line("bind x y").as_deref(), Some("bind x y\nexec vektor\n"));
        assert!(ensure_exec_line("// c\nexec vektor.cfg\n").is_none());
        assert!(ensure_exec_line("exec \"vektor\"\n").is_none());
    }

    #[test]
    fn video_merge_preserves_and_overrides() {
        let existing = "\"video.cfg\"\n{\n\t\"Version\"\t\t\"16\"\n\t\"setting.defaultres\"\t\t\"1920\"\n}\n";
        let v = VideoSettings { width: Some(1280), height: Some(960), refresh_rate: Some(240), values: BTreeMap::new() };
        let out = merge_video(Some(existing), &video_overrides(&v)).unwrap();
        let doc = vdf::parse(&out).unwrap();
        let root = doc.get("video.cfg").unwrap();
        assert_eq!(root.get_str("Version"), Some("16"));
        assert_eq!(root.get_str("setting.defaultres"), Some("1280"));
        assert_eq!(root.get_str("setting.defaultresheight"), Some("960"));
        assert_eq!(root.get_str("setting.refreshrate_numerator"), Some("240"));
        assert!(out.starts_with("\"video.cfg\""));
    }
}
