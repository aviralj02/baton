import { platform } from "@tauri-apps/plugin-os";

/**
 * Read the platform once at boot and drive both keybindings and the displayed
 * keycaps from it. Hardcoding the Cmd symbol in JSX is the mistake that makes a
 * cross-platform app feel unported.
 */
export const IS_MAC = platform() === "macos";

/**
 * What the shortcut is before `get_shortcut` answers. An accelerator rather than
 * a label, because `Shortcut` renders the caps. Mirrors `DEFAULT_SHORTCUT` in
 * settings.rs.
 */
export const DEFAULT_SHORTCUT = "CmdOrCtrl+Shift+Space";
