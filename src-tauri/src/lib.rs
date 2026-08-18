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
            app.global_shortcut().register(summon)?;

            // Window is pre-created hidden by tauri.conf.json (`visible: false`).
            // Warm the webview so the first summon is not the first paint.
            let _ = launcher::window(&app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![hide_launcher])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn hide_launcher(app: tauri::AppHandle) {
    launcher::hide(&app);
}
