//! Tauri command surface. Thin wrappers only — logic lives in the modules.
//! Blocking work runs on the blocking thread pool so the UI never stalls.

use crate::error::{Error, Result};
use crate::{appsettings, cleanup, cs2, diagnostics, display, journal, memory, nvidia, session, system, tweaks, updates};

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
