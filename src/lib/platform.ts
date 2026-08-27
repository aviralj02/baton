import { platform } from "@tauri-apps/plugin-os";

/**
 * Read the platform once at boot and drive both keybindings and the displayed
 * hint symbols from it. Hardcoding the Cmd glyph in JSX is the mistake that
 * makes a cross-platform app feel unported.
 */
export const IS_MAC = platform() === "macos";

export const ENTER_LABEL = IS_MAC ? "↵" : "Enter";

/** The summon shortcut as printed on a keycap. Mirrors `default_shortcut` in lib.rs. */
export const SUMMON_LABEL = IS_MAC ? "⌘⇧Space" : "Ctrl⇧Space";

