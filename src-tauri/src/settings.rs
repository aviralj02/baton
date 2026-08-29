//! User settings, and the summon shortcut in particular.
//!
//! Kept in `settings.json` beside the index rather than in it: the index is
//! derived and may be dropped and rebuilt at any time, and a preference that
//! vanishes on a schema change is a bug the user cannot explain.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri_plugin_global_shortcut::Shortcut;

/// Cmd+Shift+Space on macOS, Ctrl+Shift+Space elsewhere.
pub const DEFAULT_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub shortcut: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.to_string(),
        }
    }
}

/// The shortcut currently registered, so it can be unregistered when it changes.
pub struct Active(pub Mutex<Shortcut>);

fn file(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

/// Read settings, falling back to defaults on anything unreadable.
///
/// A corrupt file is not worth failing a launch over: the app is still usable
/// with the default shortcut, and the next save overwrites it.
pub fn load(dir: &Path) -> Settings {
    std::fs::read_to_string(file(dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, settings: &Settings) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(file(dir), json)
}

/// Parse an accelerator, rejecting one with no modifier.
///
/// A bare key would register globally and swallow that key in every other app.
pub fn parse(accelerator: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = accelerator
        .parse()
        .map_err(|_| format!("{accelerator} is not a shortcut Baton understands"))?;
    if shortcut.mods.is_empty() {
        return Err("A shortcut needs at least one modifier key".to_string());
    }
    Ok(shortcut)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baton-settings-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_gives_the_default_shortcut() {
        assert_eq!(load(&tmp("missing")).shortcut, DEFAULT_SHORTCUT);
    }

    #[test]
    fn a_saved_shortcut_survives_a_reload() {
        let dir = tmp("roundtrip");
        save(
            &dir,
            &Settings {
                shortcut: "Alt+Space".to_string(),
            },
        )
        .unwrap();
        assert_eq!(load(&dir).shortcut, "Alt+Space");
    }

    #[test]
    fn a_corrupt_file_falls_back_rather_than_failing() {
        let dir = tmp("corrupt");
        std::fs::write(file(&dir), "{ not json").unwrap();
        assert_eq!(load(&dir).shortcut, DEFAULT_SHORTCUT);
    }

    #[test]
    fn the_default_parses() {
        assert!(parse(DEFAULT_SHORTCUT).is_ok());
    }

    #[test]
    fn a_shortcut_without_a_modifier_is_refused() {
        // Registering this would swallow the key in every other application.
        assert!(parse("Space").is_err());
        assert!(parse("F").is_err());
    }

    #[test]
    fn nonsense_is_refused() {
        assert!(parse("Ctrl+NotAKey").is_err());
        assert!(parse("").is_err());
    }
}
