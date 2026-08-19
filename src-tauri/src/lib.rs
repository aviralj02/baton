mod launcher;
mod tray;

use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

/// Default summon shortcut. Cmd+Shift+Space on macOS, Ctrl+Shift+Space elsewhere.
/// Milestone 1 task: make this user-configurable and persist it.
fn default_shortcut() -> Shortcut {
    #[cfg(target_os = "macos")]
    let mods = Modifiers::SUPER | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let mods = Modifiers::CONTROL | Modifiers::SHIFT;

    Shortcut::new(Some(mods), Code::Space)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let summon = default_shortcut();

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            launcher::show(app);
        }));
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed && shortcut == &summon {
                        launcher::toggle(app);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // Dock-less on macOS. `skipTaskbar` in tauri.conf.json covers Windows.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::build(app)?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            // Registration can fail if another app already owns the combo.
            // Reported, not fatal: the tray still opens the launcher.
            if let Err(e) = app.global_shortcut().register(summon) {
                eprintln!("[baton] could not register {summon:?}: {e}");
            }

            launcher::configure(&app.handle());

            Ok(())
        })
        .on_window_event(|win, event| {
            // Dismiss on focus loss, the way Spotlight and Raycast do, but
            // ignore the spurious blur emitted while the window is coming
            // forward — otherwise the launcher flashes open and vanishes.
            //
            // Windows-only in practice: on macOS tauri-nspanel replaces tao's
            // window delegate, so Focused events never fire there; dismissal
            // is handled by the panel delegate in launcher::macos instead.
            if let tauri::WindowEvent::Focused(focused) = event {
                if win.label() == launcher::MAIN_WINDOW {
                    if !focused && !launcher::in_blur_grace() {
                        let _ = win.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![hide_launcher])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn hide_launcher(app: tauri::AppHandle) {
    launcher::hide(&app);
}
