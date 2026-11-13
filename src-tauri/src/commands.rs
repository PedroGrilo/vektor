//! Tauri command surface. Thin wrappers only — logic lives in the modules.
//! Blocking work runs on the blocking thread pool so the UI never stalls.

use crate::error::{Error, Result};
use crate::{appsettings, cleanup, cs2, diagnostics, display, dns, games, journal, memory, nvidia, session, startup, system, tweaks, updates, win};

async fn blocking<T, F>(f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::msg(format!("tarefa interrompida: {e}")))?
}

// ---------- system ----------

#[tauri::command]
pub async fn system_info() -> Result<system::SystemInfo> {
    blocking(system::info).await
}

#[tauri::command]
pub async fn live_stats() -> Result<system::LiveStats> {
    blocking(system::live_stats).await
}

// ---------- tweaks ----------

#[tauri::command]
pub async fn list_tweaks() -> Result<Vec<tweaks::TweakInfo>> {
    blocking(|| Ok(tweaks::list())).await
}

#[tauri::command]
pub async fn apply_tweak(id: String) -> Result<tweaks::TweakInfo> {
    blocking(move || tweaks::apply(&id)).await
}

#[tauri::command]
pub async fn revert_tweak(id: String) -> Result<tweaks::TweakInfo> {
    blocking(move || tweaks::revert(&id)).await
}

#[tauri::command]
pub async fn revert_all() -> Result<()> {
    blocking(journal::revert_all).await
}

#[tauri::command]
pub async fn clean_scan() -> Result<Vec<cleanup::CleanTarget>> {
    blocking(cleanup::scan).await
}

#[tauri::command]
pub async fn clean_run(ids: Vec<String>) -> Result<tweaks::CleanReport> {
    blocking(move || cleanup::run(&ids)).await
}

#[tauri::command]
pub async fn list_background_apps() -> Result<Vec<tweaks::ProcInfo>> {
    blocking(tweaks::processes::list_background_apps).await
}

#[tauri::command]
pub async fn kill_background_apps(names: Vec<String>) -> Result<u32> {
    blocking(move || tweaks::processes::kill_by_names(&names)).await
}

// ---------- cs2 ----------

#[tauri::command]
pub async fn cs2_detect() -> Result<cs2::Cs2Install> {
    blocking(cs2::detect).await
}

#[tauri::command]
pub async fn cs2_presets() -> Result<Vec<cs2::ConfigPreset>> {
    Ok(cs2::presets::all())
}

#[tauri::command]
pub async fn cs2_read_config(account_id: u32) -> Result<cs2::Cs2Config> {
    blocking(move || cs2::config::read(account_id)).await
}

#[tauri::command]
pub async fn cs2_apply_config(account_id: u32, config: cs2::Cs2Config) -> Result<()> {
    blocking(move || cs2::config::apply(account_id, &config)).await
}

#[tauri::command]
pub async fn cs2_launch(args: Vec<String>) -> Result<()> {
    blocking(move || cs2::launch::launch(&args)).await
}

#[tauri::command]
pub async fn cs2_recommended_launch_options() -> Result<String> {
    blocking(cs2::launch::recommended_options).await
}

#[tauri::command]
pub async fn cs2_benchmark_start() -> Result<()> {
    blocking(cs2::benchmark::start).await
}

#[tauri::command]
pub async fn cs2_benchmark_poll() -> Result<cs2::benchmark::BenchStatus> {
    blocking(cs2::benchmark::poll).await
}

// ---------- memory (standby list cleaner) ----------

#[tauri::command]
pub async fn memory_status() -> Result<memory::MemoryStatus> {
    blocking(memory::status).await
}

#[tauri::command]
pub async fn memory_set_config(config: memory::MemoryConfig) -> Result<memory::MemoryStatus> {
    blocking(move || memory::set_config(&config)).await
}

#[tauri::command]
pub async fn memory_install_service() -> Result<memory::MemoryStatus> {
    blocking(memory::install_service).await
}

#[tauri::command]
pub async fn memory_uninstall_service() -> Result<memory::MemoryStatus> {
    blocking(memory::uninstall_service).await
}

#[tauri::command]
pub async fn memory_purge_now() -> Result<memory::MemoryStatus> {
    blocking(memory::purge_now).await
}

// ---------- nvidia (Profile Inspector) ----------

#[tauri::command]
pub async fn nvidia_status(app: tauri::AppHandle) -> Result<nvidia::NvidiaStatus> {
    blocking(move || nvidia::status(&app)).await
}

#[tauri::command]
pub async fn nvidia_apply_global(app: tauri::AppHandle, options: nvidia::NvGlobalOptions) -> Result<nvidia::NvidiaStatus> {
    blocking(move || nvidia::apply_global(&app, &options)).await
}

#[tauri::command]
pub async fn nvidia_revert_global(app: tauri::AppHandle) -> Result<nvidia::NvidiaStatus> {
    blocking(move || nvidia::revert_global(&app)).await
}

#[tauri::command]
pub async fn nvidia_apply_cs2(app: tauri::AppHandle, options: nvidia::NvCs2Options) -> Result<nvidia::NvidiaStatus> {
    blocking(move || nvidia::apply_cs2(&app, &options)).await
}

#[tauri::command]
pub async fn nvidia_revert_cs2(app: tauri::AppHandle) -> Result<nvidia::NvidiaStatus> {
    blocking(move || nvidia::revert_cs2(&app)).await
}

// ---------- game session ----------

#[tauri::command]
pub async fn session_status() -> Result<session::SessionStatus> {
    blocking(session::status).await
}

#[tauri::command]
pub async fn session_set_config(config: session::SessionConfig) -> Result<session::SessionStatus> {
    blocking(move || session::set_config(config)).await
}

#[tauri::command]
pub async fn session_topology() -> Result<session::affinity::CpuTopology> {
    blocking(session::affinity::topology).await
}

#[tauri::command]
pub async fn session_stop() -> Result<session::SessionStatus> {
    blocking(session::stop).await
}

#[tauri::command]
pub async fn display_list() -> Result<Vec<display::DisplayInfo>> {
    blocking(display::modes::list).await
}

#[tauri::command]
pub async fn vibrance_info() -> Result<display::vibrance::VibranceInfo> {
    blocking(display::vibrance::info).await
}

#[tauri::command]
pub async fn vibrance_preview(level: i32) -> Result<display::vibrance::VibranceInfo> {
    blocking(move || display::vibrance::preview(level)).await
}

#[tauri::command]
pub async fn vibrance_reset() -> Result<display::vibrance::VibranceInfo> {
    blocking(display::vibrance::reset).await
}

// ---------- benchmarks ----------

#[tauri::command]
pub async fn bench_history() -> Result<Vec<cs2::history::BenchRecord>> {
    blocking(cs2::history::list).await
}

#[tauri::command]
pub async fn bench_delete(id: String) -> Result<Vec<cs2::history::BenchRecord>> {
    blocking(move || cs2::history::delete(&id)).await
}

#[tauri::command]
pub async fn bench_set_note(id: String, note: String) -> Result<Vec<cs2::history::BenchRecord>> {
    blocking(move || cs2::history::set_note(&id, &note)).await
}

// ---------- app ----------

#[tauri::command]
pub async fn app_settings() -> Result<appsettings::AppSettings> {
    blocking(appsettings::load_checked).await
}

#[tauri::command]
pub async fn app_set_settings(settings: appsettings::AppSettings) -> Result<appsettings::AppSettings> {
    blocking(move || appsettings::save(settings)).await
}

#[tauri::command]
pub async fn export_diagnostics() -> Result<String> {
    blocking(diagnostics::export).await
}

// ---------- updates ----------

#[tauri::command]
pub async fn update_check(app: tauri::AppHandle) -> Result<updates::UpdateInfo> {
    updates::check(&app).await
}

#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> Result<()> {
    updates::install(&app).await
}

// ---------- game library ----------

#[tauri::command]
pub async fn games_list(refresh: bool) -> Result<Vec<games::GameInfo>> {
    blocking(move || games::list(refresh)).await
}

#[tauri::command]
pub async fn games_set_exe(id: String, exe: String) -> Result<games::GameInfo> {
    blocking(move || games::set_exe(&id, &exe)).await
}

#[tauri::command]
pub async fn games_add(exe: String, name: Option<String>) -> Result<games::GameInfo> {
    blocking(move || games::add_manual(&exe, name)).await
}

#[tauri::command]
pub async fn games_remove(id: String) -> Result<()> {
    blocking(move || games::remove(&id)).await
}

#[tauri::command]
pub async fn games_unhide_all() -> Result<()> {
    blocking(games::unhide_all).await
}

#[tauri::command]
pub async fn games_set_tweak(id: String, kind: games::TweakKind, on: bool) -> Result<games::GameInfo> {
    blocking(move || games::set_tweak(&id, kind, on)).await
}

#[tauri::command]
pub async fn games_optimize(id: String, on: bool) -> Result<games::GameInfo> {
    blocking(move || games::optimize(&id, on)).await
}

#[tauri::command]
pub async fn games_set_session(id: String, on: bool) -> Result<games::GameInfo> {
    blocking(move || games::set_session(&id, on)).await
}

#[tauri::command]
pub async fn games_running_processes() -> Result<Vec<games::RunningProcess>> {
    blocking(games::running_processes).await
}

// ---------- restore points & change list ----------

#[tauri::command]
pub async fn restore_point_last() -> Result<Option<win::restore_point::RestorePoint>> {
    blocking(win::restore_point::last).await
}

#[tauri::command]
pub async fn restore_point_create(enable_protection: bool) -> Result<win::restore_point::RestorePointResult> {
    blocking(move || win::restore_point::create(enable_protection)).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub id: String,
    pub title: String,
}

fn describe_change(id: &str) -> String {
    if let Some(t) = tweaks::title_of(id) {
        return t.to_string();
    }
    if let Some(t) = games::describe_tweak_id(id).or_else(|| startup::describe_journal_id(id)) {
        return t;
    }
    match id {
        dns::TWEAK_ID => "Servidores DNS".into(),
        "nvidia-global" => "Perfil NVIDIA global".into(),
        "nvidia-cs2" => "Perfil NVIDIA do CS2".into(),
        "memory-service" => "Serviço VektorMemory".into(),
        "cs2-config" => "Configs do CS2".into(),
        "game-session" => "Sessão do Modo Jogo".into(),
        other => other.to_string(),
    }
}

/// Every change in the journal (catalog tweaks, games, startup, DNS, NVIDIA...).
#[tauri::command]
pub async fn list_changes() -> Result<Vec<Change>> {
    blocking(|| Ok(journal::applied_ids().into_iter().map(|id| Change { title: describe_change(&id), id }).collect())).await
}

#[tauri::command]
pub async fn revert_change(id: String) -> Result<()> {
    blocking(move || journal::revert(&id)).await
}

// ---------- startup apps ----------

#[tauri::command]
pub async fn startup_list() -> Result<Vec<startup::StartupEntry>> {
    blocking(startup::list).await
}

#[tauri::command]
pub async fn startup_set_enabled(id: String, enabled: bool) -> Result<Vec<startup::StartupEntry>> {
    blocking(move || startup::set_enabled(&id, enabled)).await
}

// ---------- network ----------

#[tauri::command]
pub async fn dns_test() -> Result<Vec<dns::DnsResult>> {
    blocking(dns::test).await
}

#[tauri::command]
pub async fn dns_status() -> Result<dns::DnsStatus> {
    blocking(dns::status).await
}

#[tauri::command]
pub async fn dns_apply(provider: String) -> Result<dns::DnsStatus> {
    blocking(move || dns::apply(&provider)).await
}

#[tauri::command]
pub async fn dns_revert() -> Result<dns::DnsStatus> {
    blocking(dns::revert).await
}
