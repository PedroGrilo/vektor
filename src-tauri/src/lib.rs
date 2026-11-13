mod appsettings;
mod cleanup;
mod commands;
mod cs2;
mod diagnostics;
mod display;
mod dns;
mod error;
mod games;
mod journal;
mod memory;
mod nvidia;
mod session;
mod startup;
mod system;
mod tray;
mod tweaks;
mod undo;
mod updates;
mod win;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // `vektor.exe --memory-service` is started by the Service Control Manager.
    if std::env::args().any(|a| a == memory::SERVICE_ARG) {
        memory::service::run_as_service();
        return;
    }
    let settings = appsettings::load();
    let start_hidden = settings.start_minimized && std::env::args().any(|a| a == "--minimized");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // Undo a game session left behind by a crash, then watch for the configured games.
            session::recover();
            session::spawn_watcher();
            tray::build(app.handle())?;
            if start_hidden && let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Keep running in the tray so the game session watcher stays active.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && appsettings::load().close_to_tray
            {
                api.prevent_close();
                tray::hide_main(window);
            }
        })
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
            commands::update_check,
            commands::update_install,
            // game library
            commands::games_list,
            commands::games_set_exe,
            commands::games_add,
            commands::games_remove,
            commands::games_unhide_all,
            commands::games_set_tweak,
            commands::games_optimize,
            commands::games_set_session,
            commands::games_running_processes,
            // restore points & changes
            commands::restore_point_last,
            commands::restore_point_create,
            commands::list_changes,
            commands::revert_change,
            // startup apps
            commands::startup_list,
            commands::startup_set_enabled,
            // network
            commands::dns_test,
            commands::dns_status,
            commands::dns_apply,
            commands::dns_revert,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vektor");
}
