mod commands;
mod db;
mod index_md;
mod launcher;
mod lint;
mod onboarding;
pub mod primer;
mod remove;
mod tray;
mod watcher;
/// Public so the reader half of the wiki can be exercised before any command
/// wires it up.
pub mod wiki;

pub const BROWSER_WINDOW: &str = "browser";

/// The wiki root. One central folder covering every project, by decision, so
/// this is neither per-repository nor configurable yet.
pub fn wiki_root(app: &tauri::AppHandle) -> std::result::Result<std::path::PathBuf, String> {
    app.path()
        .home_dir()
        .map(|home| home.join("Baton"))
        .map_err(|e| format!("no home directory: {e}"))
}

use tauri::Manager;
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

/// True only where an installed app lives; `tauri build` also outputs under `target/`.
fn is_installed() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        exe.starts_with("/Applications") || exe.starts_with(format!("{home}/Applications"))
    }

    // NSIS installs to Program Files; a per-user install lands in LocalAppData.
    #[cfg(target_os = "windows")]
    {
        ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .any(|dir| exe.starts_with(dir))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Turn on the login item once, on the very first launch.
///
/// Baton is summoned by a hotkey and has no dock icon, so an install that does
/// not come back after a reboot is indistinguishable from one that is broken:
/// the user presses the shortcut, nothing happens, and there is no window to
/// go looking in.
///
/// The marker file is what makes this a *default* rather than a policy. Without
/// it, every launch would re-enable the login item and silently overrule anyone
/// who turned it off in the tray or in System Settings.
fn default_to_launching_at_login(app: &tauri::AppHandle, data_dir: &std::path::Path) {
    use tauri_plugin_autostart::ManagerExt;

    // A login item stores an absolute path; one into target/ breaks on the next clean.
    if !is_installed() {
        return;
    }

    let marker = data_dir.join("login-item-set");
    if marker.exists() {
        return;
    }
    if let Err(e) = app.autolaunch().enable() {
        // Not fatal, and not worth a dialog: the hotkey still works for this
        // session. Leaving the marker unwritten means the next launch retries.
        eprintln!("[baton] could not add the login item: {e}");
        return;
    }
    let _ = std::fs::write(&marker, "");
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

            let dir = app.path().app_data_dir().map_err(|e| {
                format!("no app data dir: {e}")
            })?;
            std::fs::create_dir_all(&dir)?;

            // Before the tray is built, so its tick matches what we just did.
            default_to_launching_at_login(app.handle(), &dir);

            tray::build(app)?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            // Registration can fail if another app already owns the combo.
            // Reported, not fatal: the tray still opens the launcher.
            if let Err(e) = app.global_shortcut().register(summon) {
                eprintln!("[baton] could not register {summon:?}: {e}");
            }

            launcher::configure(app.handle());

            let conn = db::open(&dir.join("baton.sqlite3"))?;
            app.manage(db::Db(std::sync::Mutex::new(conn)));

            // The first sweep is disk work, so it stays off the startup path.
            // The shortcut and the tray must be live immediately, and a missing
            // ~/Baton is a degraded feature rather than a failed launch.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let swept = wiki_root(&handle).and_then(|root| {
                    // First run: the folder is Baton's own data, so it is
                    // created silently. The skill is not — that writes into
                    // another tool's config and is offered in the UI instead.
                    onboarding::ensure_wiki(&root)
                        .map_err(|e| format!("could not create {}: {e}", root.display()))?;
                    let db = handle.state::<db::Db>();
                    db::sync(&db, &root).map_err(|e| e.to_string())
                });
                // The watcher is spawned regardless of how the first sweep
                // went. A transient failure — a locked database, a path that
                // is not ready yet — used to leave the app running the whole
                // session with no watcher and no index regeneration, silently.
                if let Ok(root) = wiki_root(&handle) {
                    if let Err(e) = index_md::regenerate(&root) {
                        eprintln!("[baton] could not rewrite index.md: {e}");
                    }
                    watcher::spawn(&handle, root);
                }

                match swept {
                    Ok(report) => {
                        for error in &report.errors {
                            eprintln!("[baton] wiki page skipped: {error}");
                        }
                        // Only watch once the first index succeeded — watching a
                        // folder we could not read would emit change events
                        // against an index that was never built.
                    }
                    Err(e) => eprintln!("[baton] wiki sync failed: {e}"),
                }
            });

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
                if win.label() == launcher::MAIN_WINDOW
                    && !focused
                    && !launcher::in_blur_grace()
                {
                    let _ = win.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_launcher,
            commands::open_main_window,
            commands::sync_wiki,
            commands::rebuild_index,
            commands::wiki_status,
            commands::install_skills,
            commands::reveal_wiki,
            commands::list_projects,
            commands::search_projects,
            commands::list_pages,
            commands::search_pages,
            commands::page_backlinks,
            commands::read_page,
            commands::copy_page,
            commands::copy_primer,
            commands::delete_page,
            commands::delete_project,
            commands::delete_everything,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
