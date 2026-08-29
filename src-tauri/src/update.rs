//! Checking for and installing a new Baton.
//!
//! A hotkey app is opened, used for two seconds and dismissed. Nobody visits its
//! releases page, so without this the first build someone installs is the build
//! they keep, and that is the one most likely to be wrong.
//!
//! Every outcome is a notice rather than a dialog. A modal that interrupts a
//! summon to talk about itself is exactly the friction this app exists to avoid.

use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use crate::notice;

/// Look for a newer release, and install it if the user asked.
///
/// `announce_when_current` separates the two callers: the tray item is a
/// question the user asked and deserves an answer either way, while the startup
/// check must stay silent unless there is something to say.
pub async fn check(app: AppHandle, announce_when_current: bool) {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => {
            notice::report(&app, format!("Could not check for updates: {e}"));
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            notice::report(&app, format!("Baton {version} is downloading."));

            // No progress callbacks: the app is usable throughout, and a
            // progress bar for a 5MB download is more interruption than help.
            match update.download_and_install(|_, _| {}, || {}).await {
                Ok(()) => notice::report(
                    &app,
                    format!("Baton {version} is ready. Quit and reopen to use it."),
                ),
                Err(e) => notice::report(&app, format!("Could not install {version}: {e}")),
            }
        }
        Ok(None) => {
            if announce_when_current {
                notice::report(&app, "Baton is up to date.");
            }
        }
        Err(e) => {
            // Offline is the common case here, and it is not worth a notice on
            // a check the user did not ask for.
            if announce_when_current {
                notice::report(&app, format!("Could not check for updates: {e}"));
            } else {
                eprintln!("[baton] update check failed: {e}");
            }
        }
    }
}

/// Check once in the background, a little after launch.
///
/// Deliberately not on the startup path: the first sweep and the hotkey
/// registration matter more than this, and a network call that blocks either
/// would be felt.
pub fn check_quietly_on_launch(app: &AppHandle) {
    // Only an installed copy can replace itself; a build directory cannot.
    if !crate::is_installed() {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        // Blocking sleep on the blocking pool rather than pulling in tokio's
        // timer: this runs once, ten seconds after launch, and waits alone.
        let _ = tauri::async_runtime::spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_secs(10))
        })
        .await;
        check(handle, false).await;
    });
}
