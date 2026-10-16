mod appsettings;
mod cleanup;
mod commands;
mod cs2;
mod diagnostics;
mod display;
mod error;
mod journal;
mod memory;
mod nvidia;
mod session;
mod system;
mod tweaks;
mod undo;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // `vektor.exe --service` is started by the Service Control Manager.
    if std::env::args().any(|a| a == memory::SERVICE_ARG) {
        memory::service::run_as_service();
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // system
            commands::system_info,
            commands::live_stats,
            // tweaks
            commands::list_tweaks,
            commands::apply_tweak,
            commands::revert_tweak,
            commands::revert_all,
            commands::clean_scan,
            commands::clean_run,
            commands::list_background_apps,
            commands::kill_background_apps,
            // cs2
            commands::cs2_detect,
            commands::cs2_presets,
            commands::cs2_read_config,
            commands::cs2_apply_config,
            commands::cs2_launch,
            commands::cs2_recommended_launch_options,
            commands::cs2_benchmark_start,
            commands::cs2_benchmark_poll,
            // memory
            commands::memory_status,
            commands::memory_set_config,
            commands::memory_install_service,
            commands::memory_uninstall_service,
            commands::memory_purge_now,
            // nvidia
            commands::nvidia_status,
            commands::nvidia_apply_global,
            commands::nvidia_revert_global,
            commands::nvidia_apply_cs2,
            commands::nvidia_revert_cs2,
            // game session
            commands::session_status,
            commands::session_set_config,
            commands::session_topology,
            commands::session_stop,
            commands::display_list,
            commands::vibrance_info,
            commands::vibrance_preview,
            commands::vibrance_reset,
            // benchmarks
            commands::bench_history,
            commands::bench_delete,
            commands::bench_set_note,
            // app
            commands::app_settings,
            commands::app_set_settings,
            commands::export_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vektor");
}
