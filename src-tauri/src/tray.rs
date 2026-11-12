//! System tray icon: show the window, toggle the game session, quit.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Payload `true`/`false`: the window is on screen / hidden in the tray.
pub const VISIBLE_EVENT: &str = "vektor://visible";

/// Hide the window in the tray and let the UI pause its polling.
pub fn hide_main(window: &tauri::Window) {
    let _ = window.hide();
    let _ = window.emit(VISIBLE_EVENT, false);
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir Vektor", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop-session", "Terminar modo jogo", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &stop, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Vektor")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "stop-session" => {
                let _ = crate::session::stop();
            }
            "quit" => {
                // Leave the system as the user had it before quitting.
                let _ = crate::session::stop();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.emit(VISIBLE_EVENT, true);
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
