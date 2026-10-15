mod commands;
mod cs2;
mod error;
mod gpu;
mod journal;
mod ryzen;
mod system;
mod tweaks;
mod undo;
mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Background loop that applies vibrance while CS2 has focus.
            gpu::vibrance_watcher::spawn();
            Ok(())
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
            commands::clean_shader_cache,
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
            // gpu
            commands::gpu_status,
            commands::gpu_apply_nvidia_profile,
            commands::gpu_set_vibrance,
            commands::gpu_vibrance_auto,
            commands::gpu_set_display_mode,
            // ryzen
            commands::ryzen_status,
            commands::ryzen_apply_curve,
            commands::ryzen_clear_curve,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vektor");
}
