//! Benchmark history, stored at `%ProgramData%\Vektor\benchmarks.json`
//! (newest first, capped). The file is treated as untrusted input.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::journal;

const MAX_RECORDS: usize = 200;
const MAX_NOTE: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchRecord {
    pub id: String,
    pub date: String,
    pub avg_fps: f64,
    pub p1_fps: f64,
    pub applied_tweaks: u32,
    pub session_active: bool,
    pub note: Option<String>,
}

static LOCK: Mutex<()> = Mutex::new(());

fn path() -> PathBuf {
    journal::data_dir().join("benchmarks.json")
}

/// Parse the stored file; anything malformed or non-finite is dropped.
fn parse(bytes: &[u8]) -> Vec<BenchRecord> {
    let mut v: Vec<BenchRecord> = serde_json::from_slice(bytes).unwrap_or_default();
    v.retain(|r| r.avg_fps.is_finite() && r.p1_fps.is_finite() && !r.id.is_empty());
    for r in &mut v {
        r.note = r.note.take().map(|n| clean_note(&n)).filter(|n| !n.is_empty());
    }
    v.truncate(MAX_RECORDS);
    v
}

fn load() -> Vec<BenchRecord> {
    std::fs::read(path()).map(|b| parse(&b)).unwrap_or_default()
}

fn save(records: &[BenchRecord]) -> Result<()> {
    std::fs::create_dir_all(journal::data_dir())?;
    let tmp = path().with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(records)?)?;
    std::fs::rename(tmp, path())?;
    Ok(())
}

fn clean_note(note: &str) -> String {
    note.trim().chars().filter(|c| !c.is_control()).take(MAX_NOTE).collect()
}

fn insert(records: &mut Vec<BenchRecord>, rec: BenchRecord) {
    records.insert(0, rec);
    records.truncate(MAX_RECORDS);
}

/// Record a finished benchmark run.
pub fn add(avg: f64, p1: f64) -> Result<()> {
    if !avg.is_finite() || !p1.is_finite() {
        return Ok(());
    }
    let _g = LOCK.lock().unwrap();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let mut records = load();
    insert(
        &mut records,
        BenchRecord {
            id: format!("b{nanos}"),
            date: crate::memory::now_iso8601(),
            avg_fps: avg,
            p1_fps: p1,
            applied_tweaks: u32::try_from(journal::applied_ids().len()).unwrap_or(u32::MAX),
            session_active: journal::is_applied("game-session"),
            note: None,
        },
    );
    save(&records)
}

pub fn list() -> Result<Vec<BenchRecord>> {
    let _g = LOCK.lock().unwrap();
    Ok(load())
}

pub fn delete(id: &str) -> Result<Vec<BenchRecord>> {
    let _g = LOCK.lock().unwrap();
    let mut records = load();
    let before = records.len();
    records.retain(|r| r.id != id);
    if records.len() != before {
        save(&records)?;
    }
    Ok(records)
}

pub fn set_note(id: &str, note: &str) -> Result<Vec<BenchRecord>> {
    let _g = LOCK.lock().unwrap();
    let mut records = load();
    let note = clean_note(note);
    if let Some(r) = records.iter_mut().find(|r| r.id == id) {
        r.note = (!note.is_empty()).then_some(note);
        save(&records)?;
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str) -> BenchRecord {
        BenchRecord {
            id: id.into(),
            date: "2026-01-01T00:00:00Z".into(),
            avg_fps: 300.0,
            p1_fps: 150.0,
            applied_tweaks: 3,
            session_active: false,
            note: None,
        }
    }

    #[test]
    fn caps_and_orders_newest_first() {
        let mut v = Vec::new();
        for i in 0..(MAX_RECORDS + 5) {
            insert(&mut v, rec(&i.to_string()));
        }
        assert_eq!(v.len(), MAX_RECORDS);
        assert_eq!(v[0].id, (MAX_RECORDS + 4).to_string());
    }

    #[test]
    fn untrusted_load() {
        assert!(parse(b"not json").is_empty());
        let json = br#"[{"id":"a","date":"x","avgFps":1.0,"p1Fps":2.0,"appliedTweaks":0,"sessionActive":false,"note":"  hi\n  "},
                        {"id":"","date":"x","avgFps":1.0,"p1Fps":2.0,"appliedTweaks":0,"sessionActive":false,"note":null}]"#;
        let v = parse(json);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].note.as_deref(), Some("hi"));
    }

    #[test]
    fn note_is_trimmed_and_capped() {
        let long = "x".repeat(500);
        assert_eq!(clean_note(&long).chars().count(), MAX_NOTE);
        assert_eq!(clean_note("  ok  "), "ok");
    }
}
