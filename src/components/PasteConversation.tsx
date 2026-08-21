import { useEffect, useRef, useState } from "react";
import * as api from "../lib/api";
import { MOD_LABEL, ENTER_LABEL, hasMod } from "../lib/platform";
import type { Context } from "../types";

/**
 * The real entry point for creating a context (PRD §4 Flow A / §8 Handoff).
 *
 * Users are not expected to fill the structured fields by hand — they paste a
 * conversation and the model extracts them. The field editor in ContextDetail
 * exists for correcting the result, not for authoring it.
 */
export function PasteConversation({
  mode,
  initialName,
  onDone,
  onCancel,
  onError,
}: {
  /** "create" extracts a new context; "update" merges into an existing one. */
  mode: { kind: "create" } | { kind: "update"; id: string; name: string };
  initialName?: string;
  onDone: (c: Context) => void;
  onCancel: () => void;
  onError: (msg: string) => void;
}) {
  const [name, setName] = useState(initialName ?? "");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const areaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => areaRef.current?.focus(), []);

  const run = async () => {
    if (!text.trim() || busy) return;
    setBusy(true);
    try {
      onDone(
        mode.kind === "create"
          ? await api.createFromConversation(name, text)
          : await api.updateFromConversation(mode.id, text),
      );
    } catch (e) {
      onError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="flex h-full flex-col"
      onKeyDown={(e) => {
        if (hasMod(e) && e.key === "Enter") {
          e.preventDefault();
          void run();
        }
      }}
    >
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-black/5 px-4 py-3 dark:border-white/5"
      >
        <button
          onClick={onCancel}
          className="rounded px-1.5 py-0.5 text-sm text-neutral-400 hover:bg-black/5 dark:hover:bg-white/10"
        >
          ←
        </button>
        {mode.kind === "create" ? (
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Name (optional — inferred if blank)"
            className="flex-1 bg-transparent text-[15px] font-medium outline-none placeholder:font-normal placeholder:text-neutral-400 dark:text-neutral-100"
          />
        ) : (
          <span className="flex-1 truncate text-[15px] font-medium dark:text-neutral-100">
            Update “{mode.name}”
          </span>
        )}
      </div>

      <div className="flex-1 overflow-hidden p-3">
        <textarea
          ref={areaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          disabled={busy}
          placeholder={
            mode.kind === "create"
              ? "Paste your AI conversation here…"
              : "Paste the newer conversation — its facts win over the existing context."
          }
          className="h-full w-full resize-none rounded-md border border-black/10 bg-white/50 p-2.5 text-[13px] leading-relaxed outline-none focus:border-black/25 disabled:opacity-50 dark:border-white/10 dark:bg-black/20 dark:text-neutral-100"
        />
      </div>

      <div className="flex items-center gap-3 border-t border-black/5 px-4 py-2 text-[11px] text-neutral-400 dark:border-white/5">
        {busy ? (
          <span className="flex items-center gap-2 text-neutral-500 dark:text-neutral-300">
            <Spinner />
            Extracting context…
          </span>
        ) : (
          <>
            <button
              onClick={() => void run()}
              disabled={!text.trim()}
              className="hover:text-neutral-600 disabled:opacity-40 dark:hover:text-neutral-200"
            >
              {MOD_LABEL}
              {ENTER_LABEL} Generate
            </button>
            <span className="ml-auto">
              {text.trim() ? `${approxTokens(text).toLocaleString()} tokens` : ""}
            </span>
          </>
        )}
      </div>
    </div>
  );
}

function Spinner() {
  return (
    <span className="inline-block h-3 w-3 animate-spin rounded-full border-[1.5px] border-current border-t-transparent" />
  );
}

/**
 * Rough size indicator so a huge paste is visible before it is sent — this is
 * the only screen where the user can incur real cost. ~4 chars per token is
 * close enough to be useful and cheap enough to run on every keystroke.
 */
function approxTokens(text: string): number {
  return Math.ceil(text.length / 4);
}
