//! Disk cleanup. `scan` measures each target without deleting anything;
//! `run` cleans only the requested, known targets.
//!
//! Honest note shown in the UI: cleaning frees disk space, it does not raise
//! FPS. The exception is shader caches, which can fix stutter after a driver
//! update (the game recompiles shaders on the next run).

mod fs;

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

use self::fs::{Filter, Stats};
use crate::error::{Error, Result};
use crate::tweaks::CleanReport;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanTarget {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recommended: bool,
    pub warning: Option<String>,
    pub size_bytes: u64,
    pub files: u64,
    pub available: bool,
}

const DAY: Duration = Duration::from_secs(24 * 3600);

/// Temp folders: skip anything touched in the last 24h (running installers, open documents).
const TEMP: Filter = Filter { min_age: Some(DAY), name: None, recursive: true };
const PREFETCH: Filter = Filter { min_age: None, name: Some(|n| n.ends_with(".pf")), recursive: false };
const THUMBS: Filter = Filter {
    min_age: None,
    name: Some(|n| (n.starts_with("thumbcache_") || n.starts_with("iconcache_")) && n.ends_with(".db")),
    recursive: false,
};

enum Kind {
    /// Contents of these directories (roots are kept).
    Dirs { roots: fn() -> Vec<PathBuf>, filter: Filter },
    /// Individual files and directory contents.
    Mixed { files: fn() -> Vec<PathBuf>, roots: fn() -> Vec<PathBuf> },
    RecycleBin,
    DnsCache,
}

struct Target {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    recommended: bool,
    warning: Option<&'static str>,
    kind: Kind,
}

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

fn local() -> Option<PathBuf> {
    env_dir("LOCALAPPDATA")
}

fn windir() -> PathBuf {
    env_dir("WINDIR").or_else(|| env_dir("SystemRoot")).unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn program_data() -> PathBuf {
    env_dir("ProgramData").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

fn system_drive() -> PathBuf {
    let d = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    PathBuf::from(format!("{d}\\"))
}

fn under_local(rel: &[&str]) -> Vec<PathBuf> {
    local().map(|l| rel.iter().map(|r| l.join(r)).collect()).unwrap_or_default()
}

fn user_temp() -> Vec<PathBuf> {
    // Never let a misconfigured TEMP point the cleaner at the Windows dir or a drive root.
    env_dir("TEMP")
        .filter(|t| t.parent().is_some() && *t != windir())
        .into_iter()
        .collect()
}

static TARGETS: &[Target] = &[
    Target {
        id: "user-temp",
        title: "Ficheiros temporários do utilizador",
        description: "Conteúdo de %TEMP%. Ficheiros alterados nas últimas 24 horas são mantidos.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs { roots: user_temp, filter: TEMP },
    },
    Target {
        id: "windows-temp",
        title: "Ficheiros temporários do Windows",
        description: "Conteúdo de Windows\\Temp. Ficheiros alterados nas últimas 24 horas são mantidos.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs { roots: || vec![windir().join("Temp")], filter: TEMP },
    },
    Target {
        id: "inet-cache",
        title: "Cache de Internet do Windows",
        description: "INetCache: cache usada pelo Windows e por componentes web antigos.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs { roots: || under_local(&[r"Microsoft\Windows\INetCache"]), filter: Filter::ALL },
    },
    Target {
        id: "wer",
        title: "Relatórios de erros do Windows",
        description: "Relatórios de erros (WER) já enviados ou em fila.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs {
            roots: || {
                let wer = program_data().join(r"Microsoft\Windows\WER");
                let mut v = vec![wer.join("ReportArchive"), wer.join("ReportQueue"), wer.join("Temp")];
                v.extend(under_local(&[r"Microsoft\Windows\WER"]));
                v
            },
            filter: Filter::ALL,
        },
    },
    Target {
        id: "crash-dumps",
        title: "Dumps de falhas",
        description: "Dumps de aplicações e do sistema (CrashDumps, Minidump, MEMORY.DMP, LiveKernelReports).",
        recommended: true,
        warning: Some("Se estás a diagnosticar ecrãs azuis ou crashes, guarda os dumps antes de limpar."),
        kind: Kind::Mixed {
            files: || vec![windir().join("MEMORY.DMP")],
            roots: || {
                let mut v = under_local(&["CrashDumps"]);
                v.push(windir().join("Minidump"));
                v.push(windir().join("LiveKernelReports"));
                v
            },
        },
    },
    Target {
        id: "recycle-bin",
        title: "Reciclagem",
        description: "Esvazia a Reciclagem.",
        recommended: false,
        warning: Some("Os ficheiros apagados da Reciclagem não podem ser recuperados."),
        kind: Kind::RecycleBin,
    },
    Target {
        id: "thumbnail-cache",
        title: "Cache de miniaturas e ícones",
        description: "thumbcache/iconcache do Explorador. O Windows recria-as; ficheiros em uso são ignorados.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs { roots: || under_local(&[r"Microsoft\Windows\Explorer"]), filter: THUMBS },
    },
    Target {
        id: "prefetch",
        title: "Prefetch",
        description: "Ficheiros .pf que o Windows usa para acelerar o arranque das aplicações.",
        recommended: false,
        warning: Some("Não dá FPS. O Windows reconstrói o Prefetch e as aplicações abrem mais devagar durante uns tempos."),
        kind: Kind::Dirs { roots: || vec![windir().join("Prefetch")], filter: PREFETCH },
    },
    Target {
        id: "shader-cache-drivers",
        title: "Cache de shaders (DirectX e drivers)",
        description: "Caches de shaders do DirectX, NVIDIA, AMD e Intel. Pode resolver stutters depois de atualizar drivers; \
                      os jogos recompilam os shaders no arranque seguinte.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs {
            roots: || {
                under_local(&[
                    "D3DSCache",
                    r"NVIDIA\DXCache",
                    r"NVIDIA\GLCache",
                    r"AMD\DxCache",
                    r"AMD\DxcCache",
                    r"AMD\VkCache",
                    r"Intel\ShaderCache",
                ])
            },
            filter: Filter::ALL,
        },
    },
    Target {
        id: "steam-shader-cache",
        title: "Cache de shaders da Steam",
        description: "steamapps\\shadercache de todos os jogos. Os primeiros minutos de jogo podem ter stutter enquanto a cache é refeita.",
        recommended: false,
        warning: None,
        kind: Kind::Dirs {
            roots: || crate::cs2::steam_path().map(|s| vec![s.join(r"steamapps\shadercache")]).unwrap_or_default(),
            filter: Filter::ALL,
        },
    },
    Target {
        id: "steam-web-cache",
        title: "Cache web da Steam",
        description: "Cache do browser interno da Steam (httpcache e htmlcache).",
        recommended: false,
        warning: Some("Fecha a Steam antes de limpar."),
        kind: Kind::Dirs {
            roots: || {
                let mut v = crate::cs2::steam_path()
                    .map(|s| vec![s.join(r"appcache\httpcache")])
                    .unwrap_or_default();
                v.extend(under_local(&[r"Steam\htmlcache"]));
                v
            },
            filter: Filter::ALL,
        },
    },
    Target {
        id: "windows-update-cache",
        title: "Cache do Windows Update",
        description: "Pacotes de atualização já descarregados (SoftwareDistribution\\Download).",
        recommended: true,
        warning: None,
        kind: Kind::Dirs { roots: || vec![windir().join(r"SoftwareDistribution\Download")], filter: Filter::ALL },
    },
    Target {
        id: "delivery-optimization",
        title: "Cache de Otimização da Entrega",
        description: "Cache que o Windows usa para partilhar atualizações entre PCs.",
        recommended: true,
        warning: None,
        kind: Kind::Dirs {
            roots: || {
                vec![windir().join(
                    r"ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache",
                )]
            },
            filter: Filter::ALL,
        },
    },
    Target {
        id: "driver-installers",
        title: "Restos de instaladores de drivers",
        description: "Pastas C:\\NVIDIA e C:\\AMD onde os instaladores de drivers extraem ficheiros.",
        recommended: false,
        warning: Some("Só são precisas para reinstalar o driver sem o voltar a descarregar."),
        kind: Kind::Dirs { roots: || vec![system_drive().join("NVIDIA"), system_drive().join("AMD")], filter: Filter::ALL },
    },
    Target {
        id: "dns-cache",
        title: "Cache DNS",
        description: "Limpa a cache de resolução de nomes. Útil se um site ou servidor mudou de endereço; não liberta espaço.",
        recommended: false,
        warning: None,
        kind: Kind::DnsCache,
    },
];

fn existing(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.into_iter().filter(|p| p.exists()).collect()
}

fn measure(t: &Target) -> (Stats, bool) {
    match &t.kind {
        Kind::Dirs { roots, filter } => {
            let roots = existing(roots());
            let mut s = Stats::default();
            for r in &roots {
                let x = fs::walk(r, filter, false);
                s.bytes += x.bytes;
                s.files += x.files;
            }
            (s, !roots.is_empty())
        }
        Kind::Mixed { files, roots } => {
            let files = existing(files());
            let roots = existing(roots());
            let mut s = Stats::default();
            for f in &files {
                let x = fs::single_file(f, false);
                s.bytes += x.bytes;
                s.files += x.files;
            }
            for r in &roots {
                let x = fs::walk(r, &Filter::ALL, false);
                s.bytes += x.bytes;
                s.files += x.files;
            }
            (s, !files.is_empty() || !roots.is_empty())
        }
        Kind::RecycleBin => (platform::recycle_bin_query().unwrap_or_default(), cfg!(windows)),
        Kind::DnsCache => (Stats::default(), cfg!(windows)),
    }
}

pub fn scan() -> Result<Vec<CleanTarget>> {
    if !cfg!(windows) {
        return Err(Error::Unsupported);
    }
    Ok(TARGETS
        .iter()
        .map(|t| {
            let (s, available) = measure(t);
            CleanTarget {
                id: t.id.into(),
                title: t.title.into(),
                description: t.description.into(),
                recommended: t.recommended,
                warning: t.warning.map(Into::into),
                size_bytes: s.bytes,
                files: s.files,
                available,
            }
        })
        .collect())
}

fn steam_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case("steam.exe"))
}

fn add(report: &mut CleanReport, s: Stats, label: String) {
    report.freed_bytes += s.bytes;
    report.files_removed += u32::try_from(s.files).unwrap_or(u32::MAX);
    if s.files > 0 {
        report.paths.push(label);
    }
}

pub fn run(ids: &[String]) -> Result<CleanReport> {
    if !cfg!(windows) {
        return Err(Error::Unsupported);
    }
    let mut selected = Vec::new();
    for id in ids {
        let t = TARGETS
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::InvalidInput(format!("alvo de limpeza desconhecido: {id}")))?;
        if !selected.iter().any(|s: &&Target| s.id == t.id) {
            selected.push(t);
        }
    }
    if selected.iter().any(|t| t.id == "steam-web-cache") && steam_running() {
        return Err(Error::msg("Fecha a Steam antes de limpar a cache web da Steam."));
    }

    let mut report = CleanReport::default();
    for t in selected {
        match &t.kind {
            Kind::Dirs { roots, filter } => {
                for r in existing(roots()) {
                    let s = fs::walk(&r, filter, true);
                    add(&mut report, s, r.to_string_lossy().into_owned());
                }
            }
            Kind::Mixed { files, roots } => {
                for f in existing(files()) {
                    let s = fs::single_file(&f, true);
                    add(&mut report, s, f.to_string_lossy().into_owned());
                }
                for r in existing(roots()) {
                    let s = fs::walk(&r, &Filter::ALL, true);
                    add(&mut report, s, r.to_string_lossy().into_owned());
                }
            }
            Kind::RecycleBin => {
                let before = platform::recycle_bin_query().unwrap_or_default();
                if before.files > 0 {
                    platform::recycle_bin_empty()?;
                    add(&mut report, before, "Reciclagem".into());
                }
            }
            Kind::DnsCache => {
                platform::flush_dns()?;
                report.paths.push("Cache DNS".into());
            }
        }
    }
    Ok(report)
}

#[cfg(windows)]
mod platform {
    use super::Stats;
    use crate::error::{Error, Result};
    use windows::Win32::UI::Shell::{
        SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO,
        SHQueryRecycleBinW,
    };
    use windows::core::PCWSTR;

    /// Size of the recycle bins of all drives for the current user.
    pub fn recycle_bin_query() -> Option<Stats> {
        let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
        unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) }.ok()?;
        Some(Stats { bytes: info.i64Size.max(0) as u64, files: info.i64NumItems.max(0) as u64 })
    }

    pub fn recycle_bin_empty() -> Result<()> {
        unsafe {
            SHEmptyRecycleBinW(None, PCWSTR::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND)
        }
        .map_err(|e| Error::msg(format!("não foi possível esvaziar a Reciclagem: {e}")))
    }

    pub fn flush_dns() -> Result<()> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let status = std::process::Command::new("ipconfig.exe")
            .arg("/flushdns")
            .creation_flags(CREATE_NO_WINDOW)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::msg("ipconfig /flushdns falhou"))
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Stats;
    use crate::error::{Error, Result};

    pub fn recycle_bin_query() -> Option<Stats> {
        None
    }
    pub fn recycle_bin_empty() -> Result<()> {
        Err(Error::Unsupported)
    }
    pub fn flush_dns() -> Result<()> {
        Err(Error::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_ids_are_unique_and_known() {
        let mut ids: Vec<_> = TARGETS.iter().map(|t| t.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn filters_match_expected_names() {
        let pf = PREFETCH.name.unwrap();
        assert!(pf("chrome.exe-1a2b3c4d.pf"));
        assert!(!pf("layout.ini"));
        let th = THUMBS.name.unwrap();
        assert!(th("thumbcache_256.db"));
        assert!(th("iconcache_32.db"));
        assert!(!th("explorer.db"));
    }

    #[test]
    fn unknown_id_is_rejected() {
        if cfg!(windows) {
            assert!(run(&["nope".into()]).is_err());
        }
    }
}
