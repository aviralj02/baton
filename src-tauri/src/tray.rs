//! Menu bar (macOS) / system tray (Windows) presence.
//!
//! The tray is also the only place the login-item setting is visible. Baton has
//! no preferences window, and a hotkey app that quietly adds itself to login
//! items with no way to see or undo it is the kind of thing people uninstall.

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App,
};
use tauri_plugin_autostart::ManagerExt;

pub fn build(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Baton", true, None::<&str>)?;
    // Reading the real state rather than a stored preference: the user may have
    // removed the login item in System Settings, and the menu must not lie.
    let at_login = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start at login",
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &at_login, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => crate::launcher::show(app),
            "autostart" => {
                // The item has already flipped its own tick by the time this
                // runs, so it is the request, not the current state.
                let want = at_login.is_checked().unwrap_or(false);
                let manager = app.autolaunch();
                let result = if want { manager.enable() } else { manager.disable() };
                if let Err(e) = result {
                    eprintln!("[baton] could not set login item: {e}");
                    // Put the tick back where reality is, so the menu keeps
                    // matching the system rather than the failed request.
                    let _ = at_login.set_checked(manager.is_enabled().unwrap_or(false));
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
