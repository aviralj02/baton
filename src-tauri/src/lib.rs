mod commands;
mod db;
mod index_md;
mod launcher;
mod lint;
mod onboarding;
pub mod primer;
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

            let dir = app.path().app_data_dir().map_err(|e| {
                format!("no app data dir: {e}")
            })?;
            std::fs::create_dir_all(&dir)?;
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
                // went. A transient failure — a locked database, a path that is
                // not ready yet — used to leave the app running the whole
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
                if win.label() == launcher::MAIN_WINDOW {
                    if !focused && !launcher::in_blur_grace() {
                        let _ = win.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::hide_launcher,
            commands::open_main_window,
            commands::sync_wiki,
            commands::wiki_status,
            commands::install_skills,
            commands::reveal_wiki,
            commands::list_projects,
            commands::search_projects,
            commands::list_pages,
            commands::search_pages,
            commands::page_backlinks,
            commands::broken_links,
            commands::read_page,
            commands::copy_page,
            commands::build_primer,
            commands::copy_primer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
