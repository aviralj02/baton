import { IS_MAC } from "../lib/platform";
import { Key } from "./Key";
import { CommandIcon, ControlIcon, OptionIcon, ShiftIcon } from "./Icon";

/**
 * A keyboard shortcut, as one cap per key.
 *
 * A shortcut is several keys, so it renders as several keycaps. Squeezing them
 * into one cap ("⌘⇧Space") reads as a word rather than as three things to press,
 * and it forced the modifiers to be characters, which is the one thing an icon
 * set exists to avoid. Modifiers are drawn; everything else keeps its name.
 *
 * The accelerator is Tauri's own spelling, so this takes what `get_shortcut`
 * returns without a translation step in between.
 */
const MODIFIER_ICON: Record<string, (p: { size?: number }) => React.ReactElement> = {
  Super: CommandIcon,
  CmdOrCtrl: CommandIcon,
  Command: CommandIcon,
  Meta: CommandIcon,
  Control: ControlIcon,
  Ctrl: ControlIcon,
  Alt: OptionIcon,
  Option: OptionIcon,
  Shift: ShiftIcon,
};

/** What a modifier is called where it has no symbol printed on the key. */
const MODIFIER_WORD: Record<string, string> = {
  Super: "Win",
  CmdOrCtrl: "Ctrl",
  Command: "Win",
  Meta: "Win",
  Control: "Ctrl",
  Ctrl: "Ctrl",
  Alt: "Alt",
  Option: "Alt",
  Shift: "Shift",
};

export function Shortcut({ accelerator }: { accelerator: string }) {
  return (
    <span className="inline-flex items-center gap-1 align-middle">
      {accelerator.split("+").map((part) => (
        <Cap key={part} part={part} />
      ))}
    </span>
  );
}

function Cap({ part }: { part: string }) {
  const Icon = MODIFIER_ICON[part];

  // Windows and Linux print words on these keys, so drawing the Mac symbols
  // there would be showing someone a key their keyboard does not have.
  if (Icon && IS_MAC) {
    return (
      <Key label={part}>
        <Icon size={11} />
      </Key>
    );
  }

  return <Key>{MODIFIER_WORD[part] ?? part}</Key>;
}
