import { platform } from "@tauri-apps/plugin-os";

/**
 * Read the platform once at boot and drive both keybindings and the displayed
 * hint symbols from it. Hardcoding the Cmd glyph in JSX is the mistake that
 * makes a cross-platform app feel unported.
 */
export const IS_MAC = platform() === "macos";

export const MOD_LABEL = IS_MAC ? "⌘" : "Ctrl";
export const ENTER_LABEL = IS_MAC ? "↵" : "Enter";

/** True when the platform's primary modifier is held. */
export function hasMod(e: KeyboardEvent | React.KeyboardEvent): boolean {
  return IS_MAC ? e.metaKey : e.ctrlKey;
}
