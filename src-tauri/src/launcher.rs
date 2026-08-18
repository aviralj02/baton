//! Launcher window control.
//!
//! The window is created once at startup (hidden) and only ever shown/hidden.
//! Never create or destroy it per invocation — that costs a full webview boot
//! (300-800ms) and is the single thing that would make this feel like a web app
//! instead of a launcher.

use tauri::{AppHandle, Manager, WebviewWindow};

pub const MAIN_WINDOW: &str = "main";

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
}

pub fn show(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    let _ = win.center();
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = window(app) {
        let _ = win.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    if win.is_visible().unwrap_or(false) {
        hide(app);
    } else {
        show(app);
    }
}
