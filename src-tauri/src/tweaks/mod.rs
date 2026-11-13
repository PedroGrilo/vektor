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
    Services,
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
    /// Question shown before applying ("Usas impressora?"). Tweaks with a
    /// question are never part of the one-click boost.
    pub question: Option<String>,
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
const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";
const GPU_PREFS: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const COMPAT_LAYERS: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const FSO_FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";
const DX_GLOBAL: &str = "DirectXUserGlobalSettings";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const EDGE_POLICY: &str = r"SOFTWARE\Policies\Microsoft\Edge";

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

// ---------- per-executable tweaks (CS2 and any game in the library) ----------

/// Windows starts `exe_name` (file name only) with High CPU priority.
pub fn set_exe_high_priority(id: &str, exe_name: &str) -> Result<()> {
    let key = format!(r"{IFEO}\{exe_name}\PerfOptions");
    set_all(id, &[(Hive::Hklm, key.as_str(), "CpuPriorityClass", d(3))])
}

/// Per-app graphics preference: high-performance GPU for `exe_path` (full path).
pub fn set_exe_gpu_high_performance(id: &str, exe_path: &str) -> Result<()> {
    set_all(id, &[(Hive::Hkcu, GPU_PREFS, exe_path, sz("GpuPreference=2;"))])
}

/// Compatibility flag that turns off fullscreen optimisations for `exe_path`,
/// keeping any flags the user already set.
pub fn set_exe_fso_off(id: &str, exe_path: &str) -> Result<()> {
    let value = match registry::read(Hive::Hkcu, COMPAT_LAYERS, exe_path)? {
        Some(RegValue::Sz(s)) if s.contains(FSO_FLAG) => s,
        Some(RegValue::Sz(s)) if !s.trim().is_empty() => {
            let s = s.trim();
            if s.starts_with('~') { format!("{s} {FSO_FLAG}") } else { format!("~ {s} {FSO_FLAG}") }
        }
        _ => format!("~ {FSO_FLAG}"),
    };
    set_all(id, &[(Hive::Hkcu, COMPAT_LAYERS, exe_path, RegValue::Sz(value))])
}

/// Disable every listed service that exists, in order (dependents first).
/// A service that refuses to stop now is still disabled and stops at the next restart.
fn disable_services(id: &str, names: &[&str]) -> Result<()> {
    let mut any = false;
    for name in names {
        if services::exists(name) {
            services::disable_journaled_lenient(id, name)?;
            any = true;
        }
    }
    if any { Ok(()) } else { Err(Error::Unavailable("nenhum destes serviços existe neste sistema".into())) }
}

const NET_CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e972-e325-11ce-bfc1-08002be10318}";
/// PnPCapabilities flag that clears "Allow the computer to turn off this device to save power".
const PNP_NO_POWER_SAVING: u32 = 0x18;

fn dword_at(key: &str, name: &str) -> Option<u32> {
    match registry::read(Hive::Hklm, key, name) {
        Ok(Some(RegValue::Dword(v))) => Some(v),
        _ => None,
    }
}

/// Physical Ethernet (6) and Wi-Fi (71) adapters: their class subkeys.
fn physical_adapters() -> Vec<String> {
    registry::subkeys(Hive::Hklm, NET_CLASS)
        .into_iter()
        .filter(|k| k.len() == 4 && k.chars().all(|c| c.is_ascii_digit()))
        .map(|k| format!(r"{NET_CLASS}\{k}"))
        .filter(|key| {
            let iftype = dword_at(key, "*IfType");
            let virtual_ = dword_at(key, "Characteristics").is_some_and(|c| c & 0x1 != 0);
            matches!(iftype, Some(6) | Some(71)) && !virtual_
        })
        .collect()
}

fn nic_power_saving_off(id: &str) -> Result<()> {
    let adapters = physical_adapters();
    if adapters.is_empty() {
        return Err(Error::Unavailable("nenhuma placa de rede física encontrada".into()));
    }
    for key in &adapters {
        registry::set_journaled(id, Hive::Hklm, key, "PnPCapabilities", &d(PNP_NO_POWER_SAVING))?;
        // Energy-Efficient Ethernet / "Green Ethernet": only where the driver exposes them.
        for name in ["*EEE", "EnableGreenEthernet"] {
            if let Ok(Some(RegValue::Sz(_))) = registry::read(Hive::Hklm, key, name) {
                registry::set_journaled(id, Hive::Hklm, key, name, &sz("0"))?;
            }
        }
    }
    Ok(())
}

/// What a tweak needs to be applicable.
#[derive(Clone, Copy)]
enum Needs {
    Nothing,
    Cs2,
    Service(&'static str),
    AnyService(&'static [&'static str]),
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
    question: Option<&'static str>,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
        apply: |id| set_exe_high_priority(id, "cs2.exe"),
    },
    Tweak {
        id: "cs2-gpu-high-performance",
        category: Category::Game,
        title: "CS2 na GPU de alto desempenho",
        description: "Força o CS2 a usar a GPU dedicada (importante em portáteis com gráfica integrada + dedicada).",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Cs2,
        question: None,
        apply: |id| set_exe_gpu_high_performance(id, &cs2_exe()?),
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
        question: None,
        apply: |id| set_exe_fso_off(id, &cs2_exe()?),
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
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
        question: None,
        apply: |id| set_all(id, &[(Hive::Hklm, r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", d(5))]),
    },
    // ---- 1.1.0: background interference on current Windows 11 builds ----
    // These change visible behaviour, so they need a sign-out and are therefore
    // never part of the one-click boost (which skips anything needing a restart).
    Tweak {
        id: "accessibility-hotkeys-off",
        category: Category::Input,
        title: "Desligar atalhos de acessibilidade",
        description: "Impede que carregar 5× em Shift (Teclas Presas), manter Shift 8 s (Teclas de Filtragem) ou \
                      Num Lock 5 s (Teclas de Alternância) abra um aviso e te tire do jogo. \
                      As funcionalidades continuam disponíveis nas Definições do Windows.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        question: None,
        apply: |id| {
            // Flags with the hotkey and confirmation bits cleared (defaults: 510, 126, 62).
            set_all(id, &[
                (Hive::Hkcu, r"Control Panel\Accessibility\StickyKeys", "Flags", sz("506")),
                (Hive::Hkcu, r"Control Panel\Accessibility\Keyboard Response", "Flags", sz("122")),
                (Hive::Hkcu, r"Control Panel\Accessibility\ToggleKeys", "Flags", sz("58")),
            ])
        },
    },
    Tweak {
        id: "edge-background-off",
        category: Category::System,
        title: "Edge sem execução em segundo plano",
        description: "Desliga o Startup Boost e o modo em segundo plano do Microsoft Edge, que mantêm processos \
                      abertos depois de o fechares. O Edge demora um pouco mais a abrir e mostra \
                      “gerido pela organização” nas definições.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        question: None,
        apply: |id| {
            set_all(id, &[
                (Hive::Hklm, EDGE_POLICY, "StartupBoostEnabled", d(0)),
                (Hive::Hklm, EDGE_POLICY, "BackgroundModeEnabled", d(0)),
            ])
        },
    },
    Tweak {
        id: "widgets-off",
        category: Category::System,
        title: "Desligar Widgets (Windows 11)",
        description: "Remove o painel de Widgets (notícias e meteorologia) e os processos WebView2 que ele mantém \
                      em segundo plano. Sem efeito no Windows 10.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        question: None,
        apply: |id| set_all(id, &[(Hive::Hklm, r"SOFTWARE\Policies\Microsoft\Dsh", "AllowNewsAndInterests", d(0))]),
    },
    Tweak {
        id: "delivery-optimization-p2p-off",
        category: Category::Network,
        title: "Atualizações sem partilha P2P",
        description: "A Otimização da Entrega deixa de partilhar atualizações do Windows com outros PCs, evitando \
                      uploads em segundo plano durante o jogo. As atualizações continuam a ser transferidas \
                      normalmente; as Definições mostram “gerido pela organização”.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        question: None,
        apply: |id| {
            // 0 = HTTP only. Never 100 ("bypass"): deprecated on Windows 11 and can break downloads.
            set_all(id, &[(Hive::Hklm, r"SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization", "DODownloadMode", d(0))])
        },
    },
    // ---- 1.2.0: network adapter ----
    Tweak {
        id: "nic-power-saving-off",
        category: Category::Network,
        title: "Placa de rede sem poupança de energia",
        description: "Impede o Windows de desligar a placa de rede (cabo e Wi-Fi) para poupar energia e desliga a \
                      Ethernet de baixo consumo (EEE) onde o driver a tem. Evita quebras e picos de latência depois \
                      de algum tempo parado. Num portátil, gasta um pouco mais de bateria.",
        risk: Risk::Low,
        requires_reboot: true,
        needs: Needs::Nothing,
        question: None,
        apply: nic_power_saving_off,
    },
    // ---- 1.2.0: services, one question each ----
    // Never in the one-click boost (they have a question). Listed dependents first.
    Tweak {
        id: "svc-printing-off",
        category: Category::Services,
        title: "Desligar serviços de impressão",
        description: "Desliga o Spooler de impressão e as notificações de impressora. Deixas de poder imprimir, \
                      incluindo “Imprimir para PDF”. Digitalizar continua a funcionar.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::AnyService(&["PrintNotify", "Spooler"]),
        question: Some("Usas impressora ou “Imprimir para PDF”?"),
        apply: |id| disable_services(id, &["PrintNotify", "Spooler"]),
    },
    Tweak {
        id: "svc-fax-off",
        category: Category::Services,
        title: "Desligar o serviço de Fax",
        description: "Desliga o serviço de Fax do Windows.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Service("Fax"),
        question: Some("Envias ou recebes fax pelo PC?"),
        apply: |id| disable_services(id, &["Fax"]),
    },
    Tweak {
        id: "svc-xbox-off",
        category: Category::Services,
        title: "Desligar serviços Xbox",
        description: "Desliga a autenticação Xbox Live, os saves na nuvem, a rede Xbox e os acessórios Xbox. \
                      Jogos da Xbox app ou do Game Pass deixam de entrar na conta e de sincronizar saves, \
                      e os comandos Xbox sem fios podem deixar de funcionar.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::AnyService(&["XblAuthManager", "XblGameSave", "XboxNetApiSvc", "XboxGipSvc"]),
        question: Some("Usas a app Xbox, o Game Pass ou um comando Xbox sem fios?"),
        apply: |id| disable_services(id, &["XblGameSave", "XboxNetApiSvc", "XblAuthManager", "XboxGipSvc"]),
    },
    Tweak {
        id: "svc-bluetooth-off",
        category: Category::Services,
        title: "Desligar Bluetooth",
        description: "Desliga os serviços de Bluetooth. Auscultadores, comandos, ratos e teclados Bluetooth \
                      deixam de ligar. Dispositivos com recetor USB próprio não são afetados.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::AnyService(&["BTAGService", "BthAvctpSvc", "bthserv"]),
        question: Some("Usas algum dispositivo Bluetooth?"),
        apply: |id| disable_services(id, &["BTAGService", "BthAvctpSvc", "bthserv"]),
    },
    Tweak {
        id: "svc-maps-off",
        category: Category::Services,
        title: "Desligar gestor de mapas transferidos",
        description: "Desliga o serviço que atualiza mapas offline da app Mapas em segundo plano.",
        risk: Risk::Low,
        requires_reboot: false,
        needs: Needs::Service("MapsBroker"),
        question: Some("Usas mapas offline na app Mapas do Windows?"),
        apply: |id| disable_services(id, &["MapsBroker"]),
    },
    Tweak {
        id: "svc-search-index-off",
        category: Category::Services,
        title: "Desligar indexação da pesquisa",
        description: "Desliga o Windows Search, que indexa ficheiros e emails em segundo plano (disco e CPU). \
                      A pesquisa de ficheiros no menu Iniciar e no Explorador fica muito mais lenta, \
                      e a pesquisa do Outlook pode deixar de funcionar.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::Service("WSearch"),
        question: Some("Pesquisas ficheiros no menu Iniciar ou emails no Outlook?"),
        apply: |id| disable_services(id, &["WSearch"]),
    },
    Tweak {
        id: "svc-sysmain-off",
        category: Category::Services,
        title: "Desligar SysMain (Superfetch)",
        description: "Desliga o SysMain, que pré-carrega na RAM as apps que mais usas. Num SSD a diferença é \
                      pequena e evita uso de disco em segundo plano; num disco rígido (HDD) as apps passam \
                      a abrir mais devagar.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::Service("SysMain"),
        question: Some("O Windows está instalado num disco rígido (HDD) em vez de um SSD?"),
        apply: |id| disable_services(id, &["SysMain"]),
    },
    Tweak {
        id: "svc-biometrics-off",
        category: Category::Services,
        title: "Desligar serviço de biometria",
        description: "Desliga o Serviço Biométrico do Windows. O Windows Hello com rosto ou impressão digital \
                      deixa de funcionar; o PIN e a palavra-passe continuam.",
        risk: Risk::Medium,
        requires_reboot: false,
        needs: Needs::Service("WbioSrvc"),
        question: Some("Entras no Windows com o rosto ou a impressão digital?"),
        apply: |id| disable_services(id, &["WbioSrvc"]),
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
        Needs::AnyService(names) => {
            (!names.iter().any(|n| services::exists(n))).then(|| "Serviços não existem neste sistema".into())
        }
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
        question: t.question.map(Into::into),
    }
}

/// Title of a catalog tweak, for the Restore page.
pub fn title_of(id: &str) -> Option<&'static str> {
    TWEAKS.iter().find(|t| t.id == id).map(|t| t.title)
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
    fn service_tweaks_always_ask() {
        for t in TWEAKS.iter().filter(|t| matches!(t.category, Category::Services)) {
            assert!(t.question.is_some(), "{} has no question", t.id);
        }
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
