// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, PhysicalSize, WindowEvent,
};

use commands::AppState;

/// Show the popup, optionally anchored near a screen point (the tray icon / cursor).
fn show_popup(app: &tauri::AppHandle, view: &str, anchor: Option<PhysicalPosition<f64>>) {
    if let Some(win) = app.get_webview_window("popup") {
        let size = win.outer_size().unwrap_or(PhysicalSize::new(344, 500));
        let pos = match anchor {
            // Anchor near the tray click, just above/left of it.
            Some(p) => {
                let x = (p.x - size.width as f64).max(8.0);
                let y = (p.y - size.height as f64 - 8.0).max(8.0);
                PhysicalPosition::new(x, y)
            }
            // Fallback: bottom-right corner, above the taskbar/notification area.
            None => {
                let (mw, mh) = win
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| (m.size().width as f64, m.size().height as f64))
                    .unwrap_or((1920.0, 1080.0));
                let x = (mw - size.width as f64 - 12.0).max(8.0);
                let y = (mh - size.height as f64 - 60.0).max(8.0);
                PhysicalPosition::new(x, y)
            }
        };
        let _ = win.set_position(pos);
        let _ = win.show();
        let _ = win.set_focus();
        // Tell the UI which view to display.
        let _ = win.emit("navigate", view);
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_usage,
            commands::get_account,
            commands::get_settings,
            commands::save_settings,
            commands::detect_model,
            commands::get_history,
            commands::clear_history,
            commands::estimate,
            commands::open_url,
            commands::open_claude_code,
            commands::set_autostart,
            commands::get_autostart,
            commands::update_tray,
        ])
        .setup(|app| {
            // --- Context menu ---
            let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
            let usage = MenuItem::with_id(app, "usage", "Usage details", true, None::<&str>)?;
            let calc = MenuItem::with_id(app, "calculator", "Prompt calculator", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let open = MenuItem::with_id(app, "open_claude", "Open Claude Code", true, None::<&str>)?;
            let about = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &refresh, &usage, &calc, &settings, &sep1, &open, &about, &sep2, &quit,
                ],
            )?;

            // --- Tray icon ---
            let icon = app
                .default_window_icon()
                .cloned()
                .expect("bundled default icon");
            TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("Claude Usage — click to open")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "refresh" => show_popup(app, "usage-refresh", None),
                    "usage" => show_popup(app, "usage", None),
                    "calculator" => show_popup(app, "calculator", None),
                    "settings" => show_popup(app, "settings", None),
                    "about" => show_popup(app, "about", None),
                    "open_claude" => {
                        let _ = commands::open_claude_code();
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        show_popup(tray.app_handle(), "usage", Some(position));
                    }
                })
                .build(app)?;

            // --- Hide popup when it loses focus (click-away closes it) ---
            if let Some(win) = app.get_webview_window("popup") {
                let win_for_events = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = win_for_events.hide();
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Claude Usage Tray")
        .run(|_app, event| {
            // Keep the app alive as a pure tray app: prevent exit unless it came from the
            // Quit menu item (which calls `app.exit`, bypassing this guard).
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
