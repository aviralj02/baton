//! Launcher window control.
//!
//! The window is created once at startup (hidden) and only ever shown/hidden.
//! Never create or destroy it per invocation — that costs a full webview boot
//! (300-800ms) and is the single thing that would make this feel like a web app
//! instead of a launcher.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub const MAIN_WINDOW: &str = "main";

/// Emitted on every show. The window is created once and never remounts, so
/// this is the webview's only signal that a fresh summon happened, and it is
/// what lets the launcher re-sweep the wiki before the user starts typing.
pub const SHOWN_EVENT: &str = "launcher-shown";

static LAST_SHOWN_MS: AtomicU64 = AtomicU64::new(0);
static START: OnceLock<Instant> = OnceLock::new();

const BLUR_GRACE_MS: u64 = 400;

fn now_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// True when the window was shown too recently to trust a blur event.
pub fn in_blur_grace() -> bool {
    now_ms().saturating_sub(LAST_SHOWN_MS.load(Ordering::Relaxed)) < BLUR_GRACE_MS
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
}

pub fn configure(app: &AppHandle) {
    apply_background_effect(app);

    #[cfg(target_os = "macos")]
    macos::configure(app);

    #[cfg(not(target_os = "macos"))]
    if let Some(win) = window(app) {
        let _ = win.set_always_on_top(true);
    }
}

fn apply_background_effect(app: &AppHandle) {
    let Some(win) = window(app) else { return };

    #[cfg(target_os = "macos")]
    if let Err(e) = window_vibrancy::apply_vibrancy(
        &win,
        window_vibrancy::NSVisualEffectMaterial::Popover,
        Some(window_vibrancy::NSVisualEffectState::Active),
        Some(12.0),
    ) {
        eprintln!("[baton] vibrancy failed: {e}");
    }

    // Acrylic tint: dark, mostly transparent. UNTESTED — needs a real Windows
    // machine; see the carried-over tasks in docs/PLAN.md.
    #[cfg(target_os = "windows")]
    if let Err(e) = window_vibrancy::apply_acrylic(&win, Some((24, 24, 24, 120))) {
        eprintln!("[baton] acrylic failed: {e}");
    }
}

pub fn show(app: &AppHandle) {
    // The bar is well under 100ms from keypress to visible. Printed in dev
    // builds only, because the number is only defensible if it is measured.
    #[cfg(debug_assertions)]
    let started = std::time::Instant::now();

    LAST_SHOWN_MS.store(now_ms(), Ordering::Relaxed);
    let _ = app.emit(SHOWN_EVENT, ());

    #[cfg(target_os = "macos")]
    macos::show(app);

    #[cfg(not(target_os = "macos"))]
    if let Some(win) = window(app) {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
    }

    #[cfg(debug_assertions)]
    eprintln!("[baton] launcher shown in {:?}", started.elapsed());
}

pub fn hide(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::hide(app);

    #[cfg(not(target_os = "macos"))]
    if let Some(win) = window(app) {
        let _ = win.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    let visible = win.is_visible().unwrap_or(false);
    if visible {
        hide(app);
    } else {
        show(app);
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated, unexpected_cfgs)] // tauri-nspanel re-exports the old cocoa crate; its macro trips clippy cfgs
mod macos {
    use tauri::AppHandle;
    use tauri_nspanel::{
        cocoa::appkit::NSWindowCollectionBehavior, panel_delegate, ManagerExt, WebviewWindowExt,
    };

    const STYLE_MASK_NON_ACTIVATING_PANEL: i32 = 1 << 7;
    const FLOATING_WINDOW_LEVEL: i32 = 4;

    /// Must run on the main thread — called from `setup`.
    pub fn configure(app: &AppHandle) {
        let Some(win) = super::window(app) else {
            return;
        };
        let Ok(panel) = win.to_panel() else {
            // Without the panel the launcher still works, but summoning it
            // from a fullscreen app will kick the user back to the desktop.
            eprintln!("[baton] could not convert window to NSPanel");
            return;
        };

        panel.set_level(FLOATING_WINDOW_LEVEL);
        panel.set_style_mask(STYLE_MASK_NON_ACTIVATING_PANEL);

        panel.set_collection_behaviour(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary,
        );

        panel.set_hides_on_deactivate(false);

        // tauri-nspanel replaces tao's window delegate, so Tauri's
        // WindowEvent::Focused never fires for this window again. Dismissal on
        // focus loss is handled here instead.
        let delegate = panel_delegate!(BatonPanelDelegate {
            window_did_resign_key
        });
        let handle = app.clone();
        delegate.set_listener(Box::new(move |event: String| {
            if event.as_str() == "window_did_resign_key" && !super::in_blur_grace() {
                super::hide(&handle);
            }
        }));
        panel.set_delegate(delegate);
    }

    pub fn show(app: &AppHandle) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(win) = super::window(&handle) {
                let _ = win.center();
            }
            if let Ok(panel) = handle.get_webview_panel(super::MAIN_WINDOW) {
                panel.show();
            }
        });
    }

    pub fn hide(app: &AppHandle) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Ok(panel) = handle.get_webview_panel(super::MAIN_WINDOW) {
                panel.order_out(None);
            }
        });
    }
}
