//! Tauri command surface. Thin wrappers only — logic lives in the modules.
//! Blocking work runs on the blocking thread pool so the UI never stalls.

use crate::error::{Error, Result};
use crate::{cs2, gpu, journal, ryzen, system, tweaks};

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
pub async fn clean_shader_cache() -> Result<tweaks::CleanReport> {
    blocking(tweaks::cache::clean_shader_caches).await
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

// ---------- gpu ----------

#[tauri::command]
pub async fn gpu_status() -> Result<gpu::GpuStatus> {
    blocking(gpu::status).await
}

#[tauri::command]
pub async fn gpu_apply_nvidia_profile(preset: gpu::nvidia::NvPreset) -> Result<()> {
    blocking(move || gpu::nvidia::apply_cs2_profile(&preset)).await
}

#[tauri::command]
pub async fn gpu_set_vibrance(level: i32) -> Result<()> {
    blocking(move || gpu::set_vibrance_all(level)).await
}

#[tauri::command]
pub async fn gpu_vibrance_auto(enabled: bool, level: i32) -> Result<()> {
    gpu::vibrance_watcher::configure(enabled, level);
    Ok(())
}

#[tauri::command]
pub async fn gpu_set_display_mode(device: String, width: u32, height: u32, refresh: u32) -> Result<()> {
    blocking(move || gpu::display::apply_mode_journaled(&device, width, height, refresh)).await
}

// ---------- ryzen ----------

#[tauri::command]
pub async fn ryzen_status() -> Result<ryzen::RyzenStatus> {
    blocking(ryzen::status).await
}

#[tauri::command]
pub async fn ryzen_apply_curve(offset: i32) -> Result<ryzen::RyzenStatus> {
    blocking(move || ryzen::apply_curve(offset)).await
}

#[tauri::command]
pub async fn ryzen_clear_curve() -> Result<ryzen::RyzenStatus> {
    blocking(ryzen::clear_curve).await
}
