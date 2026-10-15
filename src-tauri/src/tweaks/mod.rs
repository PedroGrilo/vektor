//! Catalog of reversible Windows tweaks. Every change is journaled before it
//! is made, and reverting simply replays the journal for that tweak id.

pub mod processes;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::journal::{self, Hive, RegValue};
use crate::win::{power, registry, services};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Power,
    Gpu,
    System,
    Network,
    Input,
    Privacy,
    Game,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    #[allow(dead_code)] // part of the UI contract; no current tweak is high risk
    High,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakInfo {
    pub id: String,
    pub category: Category,
    pub title: String,
    pub description: String,
    pub risk: Risk,
    pub requires_reboot: bool,
    pub applied: bool,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub freed_bytes: u64,
    pub files_removed: u32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pub name: String,
    pub pid: u32,
    pub memory_mb: u64,
    pub label: String,
}

// ---------- catalog ----------

const MMCSS_PROFILE: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const MMCSS_GAMES: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games";
const CS2_IFEO: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\cs2.exe\PerfOptions";
const GPU_PREFS: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const COMPAT_LAYERS: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const FSO_FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";
const DX_GLOBAL: &str = "DirectXUserGlobalSettings";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// Set `key=value;` inside a DirectX "k=v;k=v;" settings string, keeping other entries.
fn merge_dx_setting(existing: &str, key: &str, value: &str) -> String {
    let mut out = String::new();
    let mut found = false;
    for part in existing.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let k = part.split_once('=').map_or(part, |(k, _)| k).trim();
        if k.eq_ignore_ascii_case(key) {
            if !found {
                out.push_str(&format!("{key}={value};"));
                found = true;
            }
        } else {
            out.push_str(part);
            out.push(';');
        }
    }
    if !found {
        out.push_str(&format!("{key}={value};"));
    }
    out
}

/// What a tweak needs to be applicable.
#[derive(Clone, Copy)]
enum Needs {
    Nothing,
    Cs2,
    Service(&'static str),
}

struct Tweak {
    id: &'static str,
    category: Category,
    title: &'static str,
    description: &'static str,
    risk: Risk,
    requires_reboot: bool,
    needs: Needs,
    apply: fn(&str) -> Result<()>,
}

fn d(v: u32) -> RegValue {
    RegValue::Dword(v)
}

fn sz(v: &str) -> RegValue {
    RegValue::Sz(v.to_string())
}

/// Journaled registry writes for one tweak.
fn set_all(id: &str, values: &[(Hive, &str, &str, RegValue)]) -> Result<()> {
    for (hive, key, name, value) in values {
        registry::set_journaled(id, *hive, key, name, value)?;
    }
    Ok(())
}

fn cs2_exe() -> Result<String> {
    crate::cs2::game_exe_path()
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Unavailable("instalação do CS2 não encontrada".into()))
}

static TWEAKS: &[Tweak] = &[
    Tweak {
        id: "power-plan",
        category: Category::Power,
        title: "Plano de energia de alto desempenho",
        description: "Cria e ativa o plano \"Vektor Performance\" (baseado no Desempenho Máximo do Windows): \
                      o CPU deixa de reduzir a frequência agressivamente. Aumenta o consumo e a temperatura em repouso.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| power::activate_vektor_plan(id),
    },
    Tweak {
        id: "game-mode",
        category: Category::System,
        title: "Ativar Modo de Jogo",
        description: "Ativa o Modo de Jogo do Windows, que dá prioridade ao jogo e evita atualizações em segundo plano durante as sessões.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[
                (Hive::Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled", d(1)),
                (Hive::Hkcu, r"Software\Microsoft\GameBar", "AllowAutoGameMode", d(1)),
            ])
        },
    },
    Tweak {
        id: "game-dvr-off",
        category: Category::System,
        title: "Desligar gravação em segundo plano (Game DVR)",
        description: "Desliga a captura automática da Xbox Game Bar, que grava o ecrã em segundo plano e consome GPU.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[
                (Hive::Hkcu, r"System\GameConfigStore", "GameDVR_Enabled", d(0)),
                (Hive::Hkcu, r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", d(0)),
            ])
        },
    },
    Tweak {
        id: "hags",
        category: Category::Gpu,
        title: "Agendamento de GPU acelerado por hardware",
        description: "Ativa o HAGS, que passa o agendamento de tarefas da GPU para a própria placa. \
                      Requer GPU e driver compatíveis (WDDM 2.7+). Em alguns sistemas pode piorar a latência: testar com benchmark.",
        risk: Risk::Medium,
        requires_reboot: true,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[(Hive::Hklm, r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", d(2))])
        },
    },
    Tweak {
        id: "mmcss-games",
        category: Category::System,
        title: "Prioridade multimédia para jogos",
        description: "Ajusta o agendador multimédia (MMCSS): mais prioridade de CPU e GPU para tarefas de jogo \
                      e menos tempo reservado a tarefas de fundo (SystemResponsiveness 10).",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[
                (Hive::Hklm, MMCSS_PROFILE, "SystemResponsiveness", d(10)),
                (Hive::Hklm, MMCSS_GAMES, "GPU Priority", d(8)),
                (Hive::Hklm, MMCSS_GAMES, "Priority", d(6)),
                (Hive::Hklm, MMCSS_GAMES, "Scheduling Category", sz("High")),
                (Hive::Hklm, MMCSS_GAMES, "SFIO Priority", sz("High")),
            ])
        },
    },
    Tweak {
        id: "network-throttling-off",
        category: Category::Network,
        title: "Desligar limitação de rede multimédia",
        description: "Remove o limite que o Windows impõe ao processamento de pacotes de rede enquanto há reprodução multimédia.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| set_all(id, &[(Hive::Hklm, MMCSS_PROFILE, "NetworkThrottlingIndex", d(0xFFFF_FFFF))]),
    },
    Tweak {
        id: "priority-separation",
        category: Category::System,
        title: "Dar prioridade à janela em primeiro plano",
        description: "Define Win32PrioritySeparation = 0x26: intervalos de tempo curtos e variáveis com bónus para a aplicação em primeiro plano.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[(
                Hive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\PriorityControl",
                "Win32PrioritySeparation",
                d(0x26),
            )])
        },
    },
    Tweak {
        id: "mouse-accel-off",
        category: Category::Input,
        title: "Desligar aceleração do rato",
        description: "Desliga \"Melhorar precisão do ponteiro\": o movimento do cursor passa a ser 1:1 com o rato. \
                      Tem efeito depois de terminar sessão.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[
                (Hive::Hkcu, r"Control Panel\Mouse", "MouseSpeed", sz("0")),
                (Hive::Hkcu, r"Control Panel\Mouse", "MouseThreshold1", sz("0")),
                (Hive::Hkcu, r"Control Panel\Mouse", "MouseThreshold2", sz("0")),
            ])
        },
    },
    Tweak {
        id: "cs2-high-priority",
        category: Category::Game,
        title: "CS2 com prioridade de CPU alta",
        description: "O Windows arranca o cs2.exe sempre com prioridade Alta (Image File Execution Options).",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| set_all(id, &[(Hive::Hklm, CS2_IFEO, "CpuPriorityClass", d(3))]),
    },
    Tweak {
        id: "cs2-gpu-high-performance",
        category: Category::Game,
        title: "CS2 na GPU de alto desempenho",
        description: "Força o CS2 a usar a GPU dedicada (importante em portáteis com gráfica integrada + dedicada).",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Cs2,
        apply: |id| {
            let exe = cs2_exe()?;
            set_all(id, &[(Hive::Hkcu, GPU_PREFS, exe.as_str(), sz("GpuPreference=2;"))])
        },
    },
    Tweak {
        id: "cs2-fso-off",
        category: Category::Game,
        title: "Desligar otimizações de ecrã inteiro no CS2",
        description: "Faz o CS2 usar ecrã inteiro exclusivo em vez do modo híbrido do Windows. \
                      Em alguns sistemas reduz a latência; noutros não faz diferença.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Cs2,
        apply: |id| {
            let exe = cs2_exe()?;
            // Keep any compatibility flags the user already set.
            let value = match registry::read(Hive::Hkcu, COMPAT_LAYERS, &exe)? {
                Some(RegValue::Sz(s)) if s.contains(FSO_FLAG) => s,
                Some(RegValue::Sz(s)) if !s.trim().is_empty() => {
                    let s = s.trim();
                    if s.starts_with('~') { format!("{s} {FSO_FLAG}") } else { format!("~ {s} {FSO_FLAG}") }
                }
                _ => format!("~ {FSO_FLAG}"),
            };
            set_all(id, &[(Hive::Hkcu, COMPAT_LAYERS, exe.as_str(), RegValue::Sz(value))])
        },
    },
    Tweak {
        id: "telemetry-off",
        category: Category::Privacy,
        title: "Desligar serviço de telemetria",
        description: "Desativa o serviço \"Experiências de Utilizador Ligado e Telemetria\" (DiagTrack), \
                      que envia dados de diagnóstico para a Microsoft e usa disco/CPU em segundo plano.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Service("DiagTrack"),
        apply: |id| services::disable_journaled(id, "DiagTrack"),
    },
    Tweak {
        id: "power-throttling-off",
        category: Category::Power,
        title: "Desligar Power Throttling",
        description: "Impede o Windows de limitar a CPU de processos que considera em segundo plano. \
                      Útil sobretudo em portáteis; aumenta o consumo de energia.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[(
                Hive::Hklm,
                r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
                "PowerThrottlingOff",
                d(1),
            )])
        },
    },
    Tweak {
        id: "windowed-game-optimizations",
        category: Category::Gpu,
        title: "Otimizações para jogos em janela",
        description: "Ativa o modelo de apresentação flip para jogos DirectX 10/11 em janela ou sem margens, \
                      com latência semelhante ao ecrã inteiro (Windows 11). Mantém as outras definições gráficas do utilizador.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            let current = match registry::read(Hive::Hkcu, GPU_PREFS, DX_GLOBAL)? {
                Some(RegValue::Sz(s)) => s,
                _ => String::new(),
            };
            let merged = merge_dx_setting(&current, "SwapEffectUpgradeEnable", "1");
            set_all(id, &[(Hive::Hkcu, GPU_PREFS, DX_GLOBAL, RegValue::Sz(merged))])
        },
    },
    Tweak {
        id: "visual-effects-performance",
        category: Category::System,
        title: "Efeitos visuais para desempenho",
        description: "Define os efeitos visuais do Windows para \"Ajustar para obter o melhor desempenho\" (animações e sombras desligadas). \
                      Torna o ambiente de trabalho mais leve; não muda o FPS dentro dos jogos. Tem efeito ao terminar sessão.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[(
                Hive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
                "VisualFXSetting",
                d(2),
            )])
        },
    },
    Tweak {
        id: "transparency-off",
        category: Category::System,
        title: "Desligar efeitos de transparência",
        description: "Desliga a transparência da barra de tarefas e menus. Ganho pequeno, sobretudo em GPUs integradas.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| set_all(id, &[(Hive::Hkcu, PERSONALIZE, "EnableTransparency", d(0))]),
    },
    Tweak {
        id: "background-apps-off",
        category: Category::System,
        title: "Bloquear apps da Store em segundo plano",
        description: "Impede as aplicações da Microsoft Store de correrem em segundo plano. \
                      Notificações dessas apps (ex.: Mail) podem deixar de chegar com a app fechada.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Nothing,
        apply: |id| {
            set_all(id, &[(
                Hive::Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
                d(1),
            )])
        },
    },
    Tweak {
        id: "mpo-off",
        category: Category::Gpu,
        title: "Desligar Multiplane Overlay (MPO)",
        description: "Correção para flicker ou stutter em algumas configurações com vários monitores e GPU NVIDIA. \
                      Só aplicar se tiveres esse problema; versões recentes do Windows podem ignorar esta definição.",
        risk: Risk::Medium,
        requires_reboot: true,
        needs: Needs::Nothing,
        apply: |id| set_all(id, &[(Hive::Hklm, r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", d(5))]),
    },
];

fn find(id: &str) -> Result<&'static Tweak> {
    TWEAKS
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| Error::InvalidInput(format!("otimização desconhecida: {id}")))
}

fn unavailable_reason(t: &Tweak) -> Option<String> {
    if !cfg!(windows) {
        return Some("Disponível apenas no Windows".into());
    }
    match t.needs {
        Needs::Nothing => None,
        Needs::Cs2 => crate::cs2::game_exe_path()
            .is_none()
            .then(|| "CS2 não encontrado neste PC".into()),
        Needs::Service(name) => (!services::exists(name)).then(|| format!("Serviço {name} não existe neste sistema")),
    }
}

fn info(t: &Tweak) -> TweakInfo {
    let reason = unavailable_reason(t);
    TweakInfo {
        id: t.id.into(),
        category: t.category,
        title: t.title.into(),
        description: t.description.into(),
        risk: t.risk,
        requires_reboot: t.requires_reboot,
        applied: journal::is_applied(t.id),
        available: reason.is_none(),
        reason,
    }
}

pub fn list() -> Vec<TweakInfo> {
    TWEAKS.iter().map(info).collect()
}

pub fn apply(id: &str) -> Result<TweakInfo> {
    let t = find(id)?;
    if journal::is_applied(t.id) {
        return Ok(info(t));
    }
    if let Some(r) = unavailable_reason(t) {
        return Err(Error::Unavailable(r));
    }
    if let Err(e) = (t.apply)(t.id) {
        // Roll back whatever part of the tweak was already applied.
        let _ = journal::revert(t.id);
        return Err(e);
    }
    Ok(info(t))
}

pub fn revert(id: &str) -> Result<TweakInfo> {
    let t = find(id)?;
    journal::revert(t.id)?;
    Ok(info(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dx_merge_adds_to_empty() {
        assert_eq!(merge_dx_setting("", "SwapEffectUpgradeEnable", "1"), "SwapEffectUpgradeEnable=1;");
    }

    #[test]
    fn dx_merge_keeps_other_entries() {
        assert_eq!(
            merge_dx_setting("VRROptimizeEnable=0;", "SwapEffectUpgradeEnable", "1"),
            "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;"
        );
    }

    #[test]
    fn dx_merge_replaces_existing_value() {
        assert_eq!(
            merge_dx_setting("SwapEffectUpgradeEnable=0;VRROptimizeEnable=1", "SwapEffectUpgradeEnable", "1"),
            "SwapEffectUpgradeEnable=1;VRROptimizeEnable=1;"
        );
        assert_eq!(
            merge_dx_setting("swapeffectupgradeenable=0;;SwapEffectUpgradeEnable=0;", "SwapEffectUpgradeEnable", "1"),
            "SwapEffectUpgradeEnable=1;"
        );
    }

    #[test]
    fn tweak_ids_unique() {
        let mut ids: Vec<_> = TWEAKS.iter().map(|t| t.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
