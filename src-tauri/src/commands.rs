//! Tauri command surface. Thin wrappers only — logic lives in the modules.
//! Blocking work runs on the blocking thread pool so the UI never stalls.

use crate::error::{Error, Result};
use crate::{cleanup, cs2, journal, memory, nvidia, system, tweaks};

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
