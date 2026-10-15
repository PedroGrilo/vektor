//! FPS benchmark using the public Workshop "FPS Benchmark" map. CS2 is started
//! with `-condebug` so its console is mirrored to `game/csgo/console.log`, and
//! the map prints a `[VProf] FPS: Avg=…, P1=…` line when the run ends.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{Error, Result};

pub const WORKSHOP_ID: &str = "3240880604";
const WORKSHOP_MAP: &str = "de_dust2";
const BENCH_CFG: &str = "vektor_benchmark";
const TIMEOUT: Duration = Duration::from_secs(6 * 60);
/// Grace period for Steam to actually start cs2.exe.
const START_GRACE: Duration = Duration::from_secs(90);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BenchState {
    Idle,
    WaitingForGame,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchStatus {
    pub state: BenchState,
    pub avg_fps: Option<f64>,
    pub p1_fps: Option<f64>,
    pub message: Option<String>,
}

struct Run {
    started: Instant,
    log: PathBuf,
    cfg: PathBuf,
    seen_game: bool,
    finished: Option<BenchStatus>,
}

static RUN: Mutex<Option<Run>> = Mutex::new(None);

fn status(state: BenchState, msg: Option<&str>) -> BenchStatus {
    BenchStatus { state, avg_fps: None, p1_fps: None, message: msg.map(str::to_string) }
}

/// Find the last `[VProf] FPS: Avg=X, P1=Y` in the log text.
pub fn parse_result(log: &str) -> Option<(f64, f64)> {
    log.lines().rev().find_map(|line| {
        let rest = &line[line.find("[VProf] FPS:")? + "[VProf] FPS:".len()..];
        let avg = num_after(rest, "Avg=")?;
        let p1 = num_after(rest, "P1=")?;
        Some((avg, p1))
    })
}

fn num_after(s: &str, key: &str) -> Option<f64> {
    let start = s.find(key)? + key.len();
    let n: String = s[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    n.parse().ok()
}

fn workshop_dir() -> Result<PathBuf> {
    let (lib, _) = super::find_game().ok_or_else(|| Error::Unavailable("instalação do CS2 não encontrada".into()))?;
    Ok(lib.join("steamapps").join("workshop").join("content").join(super::APP_ID).join(WORKSHOP_ID))
}

fn open_workshop_page() {
    #[cfg(windows)]
    {
        // Fixed string only — no user input reaches the shell.
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", "steam://url/CommunityFilePage/3240880604"])
            .spawn();
    }
}

pub fn start() -> Result<()> {
    if super::is_running() {
        return Err(Error::msg("Fecha o CS2 antes de iniciar o benchmark."));
    }
    let ws = workshop_dir()?;
    let has_map = std::fs::read_dir(&ws).map(|mut d| d.next().is_some()).unwrap_or(false);
    if !has_map {
        open_workshop_page();
        return Err(Error::Unavailable(
            "o mapa de benchmark não está instalado. Abrimos a página da Workshop no Steam: \
             clica em \"Subscrever\", espera que o download termine e tenta de novo."
                .into(),
        ));
    }

    let csgo = super::csgo_dir()?;
    let log = csgo.join("console.log");
    if log.exists() {
        let _ = std::fs::rename(&log, csgo.join("console.log.prev"));
    }
    let cfg = csgo.join("cfg").join(format!("{BENCH_CFG}.cfg"));
    std::fs::write(&cfg, format!("// Gerado pelo Vektor (benchmark)\nmap_workshop {WORKSHOP_ID} {WORKSHOP_MAP}\n"))?;

    let args: Vec<String> = ["-novid", "-condebug", "+fps_max", "0", "+exec", BENCH_CFG]
        .iter()
        .map(|s| s.to_string())
        .collect();
    if let Err(e) = super::launch::launch(&args) {
        let _ = std::fs::remove_file(&cfg);
        return Err(e);
    }

    *RUN.lock().unwrap() = Some(Run { started: Instant::now(), log, cfg, seen_game: false, finished: None });
    Ok(())
}

pub fn poll() -> Result<BenchStatus> {
    let mut guard = RUN.lock().unwrap();
    let Some(run) = guard.as_mut() else {
        return Ok(status(BenchState::Idle, None));
    };
    if let Some(done) = &run.finished {
        return Ok(done.clone());
    }

    let text = std::fs::read(&run.log).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    let running = super::is_running();
    run.seen_game |= running;

    let result = if let Some((avg, p1)) = parse_result(&text) {
        Some(BenchStatus {
            state: BenchState::Done,
            avg_fps: Some(avg),
            p1_fps: Some(p1),
            message: Some("Benchmark concluído.".into()),
        })
    } else if run.started.elapsed() > TIMEOUT {
        Some(status(BenchState::Failed, Some("O benchmark excedeu o tempo limite (6 min).")))
    } else if run.seen_game && !running {
        Some(status(BenchState::Failed, Some("O CS2 fechou antes de o benchmark terminar.")))
    } else if !run.seen_game && run.started.elapsed() > START_GRACE {
        Some(status(BenchState::Failed, Some("O CS2 não arrancou. Verifica se o Steam está aberto.")))
    } else {
        None
    };

    match result {
        Some(r) => {
            let _ = std::fs::remove_file(&run.cfg);
            run.finished = Some(r.clone());
            Ok(r)
        }
        None if run.seen_game => Ok(status(BenchState::Running, Some("A medir FPS…"))),
        None => Ok(status(BenchState::WaitingForGame, Some("A aguardar que o CS2 arranque…"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vprof_line() {
        let log = "Loading map\n[VProf] FPS: Avg=412.5, P1=201.3\nother\n";
        assert_eq!(parse_result(log), Some((412.5, 201.3)));
        assert_eq!(parse_result("nothing here"), None);
        let two = "[VProf] FPS: Avg=100, P1=50\n[VProf] FPS: Avg=300.0, P1=150.5\n";
        assert_eq!(parse_result(two), Some((300.0, 150.5)));
    }
}
