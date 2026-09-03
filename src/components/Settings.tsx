import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { Shortcut } from "./Shortcut";
import { Button } from "./Button";

/**
 * Settings, of which there is currently one.
 *
 * The shortcut is recorded rather than typed. An accelerator string is a thing
 * the app understands and the user does not, and asking someone to write
 * "CmdOrCtrl+Shift+Space" is asking them to learn our vocabulary to change a
 * keystroke.
 */
export function Settings({ onNotice }: { onNotice: (message: string) => void }) {
  const [shortcut, setShortcut] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    api
      .getShortcut()
      .then(setShortcut)
      .catch((e) => onNotice(String(e)));
  }, [onNotice]);

  // Held open until a full combination arrives, so a lone modifier does not
  // cancel the recording the moment the user reaches for the second key.
  useEffect(() => {
    if (!recording) return;

    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const accelerator = toAccelerator(e);
      if (!accelerator) return;

      setRecording(false);
      try {
        await api.setShortcut(accelerator);
        setShortcut(accelerator);
      } catch (err) {
        onNotice(String(err));
      }
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onNotice]);

  return (
    <div className="mx-auto flex h-full max-w-lg flex-col justify-center px-8">
      <h1 className="font-serif text-display tracking-tight text-ink">Settings</h1>

      <section className="mt-6 flex items-start justify-between gap-6 border-t border-line pt-5">
        <div className="min-w-0">
          <h2 className="text-ui font-medium text-ink">Summon Baton</h2>
          <p className="mt-1.5 text-ui leading-relaxed text-body">
            {recording
              ? "Press the combination you want. Escape to cancel."
              : "Works from any app, including over a fullscreen window."}
          </p>
        </div>

        <Button
          variant={recording ? "armed" : "quiet"}
          pressed={recording}
          onClick={() => setRecording((v) => !v)}
        >
          {recording ? (
            "Listening…"
          ) : shortcut ? (
            <Shortcut accelerator={shortcut} />
          ) : (
            "…"
          )}
        </Button>
      </section>
    </div>
  );
}

/**
 * A DOM keydown as a Tauri accelerator, or null while the chord is incomplete.
 *
 * Modifier-only presses return null so the listener keeps waiting: the user is
 * mid-chord, not finished.
 */
function toAccelerator(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.metaKey) mods.push("Super");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");

  const key = namedKey(e);
  if (!key || mods.length === 0) return null;
  return [...mods, key].join("+");
}

/** The key half of an accelerator, in the spelling Tauri parses. */
function namedKey(e: KeyboardEvent): string | null {
  if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) return null;
  if (e.code.startsWith("Key")) return e.code.slice(3);
  if (e.code.startsWith("Digit")) return e.code.slice(5);
  if (/^F\d{1,2}$/.test(e.code)) return e.code;
  const named: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backquote: "Backquote",
    Minus: "Minus",
    Equal: "Equal",
    BracketLeft: "BracketLeft",
    BracketRight: "BracketRight",
    Backslash: "Backslash",
    Semicolon: "Semicolon",
    Quote: "Quote",
    Comma: "Comma",
    Period: "Period",
    Slash: "Slash",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };
  return named[e.code] ?? null;
}
