import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { MOD_LABEL, ENTER_LABEL, hasMod } from "../lib/platform";
import type { Context, ContextBody } from "../types";

export function ContextDetail({
  context,
  onBack,
  onCopy,
  onUpdate,
  onHandoff,
  onSaved,
  onDeleted,
  onError,
}: {
  context: Context;
  onBack: () => void;
  onCopy: () => void;
  /** Paste a newer conversation and re-derive (PRD §9). Optional: the browser
   *  window offers it, but a caller may omit it. */
  onUpdate?: () => void;
  /** Rewrite the context as a prompt for the next model (PRD §8). */
  onHandoff?: () => void;
  onSaved: (c: Context) => void;
  onDeleted: () => void;
  onError: (msg: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(context.name);
  const [draft, setDraft] = useState<Context>(context);
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    setDraft(context);
    setName(context.name);
    setEditing(false);
    setConfirmDelete(false);
  }, [context]);

  const save = async () => {
    try {
      const body = toBody(draft);
      onSaved(await api.saveContext({ id: context.id, name, body }));
      setEditing(false);
    } catch (e) {
      onError(String(e));
    }
  };

  const remove = async () => {
    try {
      await api.deleteContext(context.id);
      onDeleted();
    } catch (e) {
      onError(String(e));
    }
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!hasMod(e)) return;

      const el = e.target as HTMLElement | null;
      const typing =
        el?.tagName === "TEXTAREA" ||
        el?.tagName === "INPUT" ||
        el?.isContentEditable === true;

      const k = e.key.toLowerCase();
      if (k === "enter") {
        // Save/copy stays available while typing — that is the point of it.
        e.preventDefault();
        if (editing) void save();
        else onCopy();
      } else if (k === "u" && !typing && onUpdate) {
        e.preventDefault();
        onUpdate();
      } else if (k === "e" && !typing) {
        e.preventDefault();
        setEditing((v) => !v);
      } else if (k === "backspace" && !typing) {
        e.preventDefault();
        setConfirmDelete(true);
      }
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
    // `save` closes over draft/name, so it must be in the dep list or the
    // handler would persist a stale draft.
  }, [editing, draft, name, onCopy, onUpdate, context.id]);

  return (
    <div className="flex h-full flex-col">
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-black/5 px-4 py-3 dark:border-white/5"
      >
        <button
          onClick={onBack}
          className="rounded px-1.5 py-0.5 text-sm text-neutral-400 hover:bg-black/5 dark:hover:bg-white/10"
        >
          ←
        </button>
        {editing ? (
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="flex-1 bg-transparent text-[15px] font-medium outline-none dark:text-neutral-100"
          />
        ) : (
          <h1 className="flex-1 truncate text-[15px] font-medium dark:text-neutral-100">
            {context.name}
          </h1>
        )}
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3">
        {confirmDelete ? (
          <div className="rounded-lg border border-red-500/30 bg-red-500/5 p-4">
            <p className="text-sm dark:text-neutral-200">
              Delete “{context.name}”? This cannot be undone.
            </p>
            <div className="mt-3 flex gap-2">
              <button
                onClick={() => void remove()}
                className="rounded-md bg-red-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700"
              >
                Delete
              </button>
              <button
                onClick={() => setConfirmDelete(false)}
                className="rounded-md px-3 py-1.5 text-xs hover:bg-black/5 dark:text-neutral-300 dark:hover:bg-white/10"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : editing ? (
          <Editor draft={draft} setDraft={setDraft} />
        ) : (
          <Viewer context={context} />
        )}
      </div>

      <div className="flex items-center gap-3 border-t border-black/5 px-4 py-2 text-[11px] text-neutral-400 dark:border-white/5">
        {editing ? (
          <>
            <button onClick={() => void save()} className="hover:text-neutral-600 dark:hover:text-neutral-200">
              {MOD_LABEL}{ENTER_LABEL} Save
            </button>
            <button onClick={() => setEditing(false)} className="hover:text-neutral-600 dark:hover:text-neutral-200">
              Cancel
            </button>
          </>
        ) : (
          <>
            <button onClick={onCopy} className="hover:text-neutral-600 dark:hover:text-neutral-200">
              {MOD_LABEL}{ENTER_LABEL} Copy context
            </button>
            <button onClick={() => setEditing(true)} className="hover:text-neutral-600 dark:hover:text-neutral-200">
              {MOD_LABEL}E Edit
            </button>
            {onHandoff && (
              <button
                onClick={onHandoff}
                className="hover:text-neutral-600 dark:hover:text-neutral-200"
              >
                Handoff
              </button>
            )}
            {onUpdate && (
              <button
                onClick={onUpdate}
                className="hover:text-neutral-600 dark:hover:text-neutral-200"
              >
                {MOD_LABEL}U Update
              </button>
            )}
            <button
              onClick={() => setConfirmDelete(true)}
              className="ml-auto hover:text-red-500"
            >
              Delete
            </button>
          </>
        )}
      </div>
    </div>
  );
}

function Viewer({ context }: { context: Context }) {
  const sections: [string, string[] | string | null][] = [
    ["Goal", context.goal],
    ["Current State", context.currentState],
    ["Decisions", context.decisions],
    ["Things Tried", context.tried],
    ["Constraints", context.constraints],
    ["Relevant Files", context.relevantFiles],
    ["Open Issues", context.openIssues],
    ["Next Steps", context.nextSteps],
  ];
  const populated = sections.filter(([, v]) =>
    Array.isArray(v) ? v.length > 0 : Boolean(v?.trim()),
  );

  if (populated.length === 0) {
    return (
      <p className="py-6 text-center text-sm text-neutral-400">
        Empty context. Press {MOD_LABEL}E to add details.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      {populated.map(([heading, value]) => (
        <section key={heading}>
          <h2 className="mb-1 text-[11px] font-medium uppercase tracking-wide text-neutral-400">
            {heading}
          </h2>
          {Array.isArray(value) ? (
            <ul className="space-y-0.5">
              {value.map((v, i) => (
                <li key={i} className="text-sm text-neutral-700 dark:text-neutral-200">
                  • {v}
                </li>
              ))}
            </ul>
          ) : (
            <p className="whitespace-pre-wrap text-sm text-neutral-700 dark:text-neutral-200">
              {value}
            </p>
          )}
        </section>
      ))}
    </div>
  );
}

function Editor({
  draft,
  setDraft,
}: {
  draft: Context;
  setDraft: (c: Context) => void;
}) {
  const text = (label: string, key: "goal" | "currentState") => (
    <Field label={label} key={key}>
      <textarea
        rows={2}
        value={draft[key] ?? ""}
        onChange={(e) => setDraft({ ...draft, [key]: e.target.value || null })}
        className="w-full resize-y rounded-md border border-black/10 bg-white/50 px-2 py-1.5 text-sm outline-none focus:border-black/25 dark:border-white/10 dark:bg-black/20 dark:text-neutral-100"
      />
    </Field>
  );

  const list = (
    label: string,
    key: "decisions" | "tried" | "constraints" | "relevantFiles" | "openIssues" | "nextSteps",
  ) => (
    <Field label={`${label} (one per line)`} key={key}>
      <textarea
        rows={3}
        value={draft[key].join("\n")}
        onChange={(e) =>
          setDraft({ ...draft, [key]: e.target.value.split("\n") })
        }
        className="w-full resize-y rounded-md border border-black/10 bg-white/50 px-2 py-1.5 text-sm outline-none focus:border-black/25 dark:border-white/10 dark:bg-black/20 dark:text-neutral-100"
      />
    </Field>
  );

  return (
    <div className="space-y-3">
      {text("Goal", "goal")}
      {text("Current State", "currentState")}
      {list("Decisions", "decisions")}
      {list("Things Tried", "tried")}
      {list("Constraints", "constraints")}
      {list("Relevant Files", "relevantFiles")}
      {list("Open Issues", "openIssues")}
      {list("Next Steps", "nextSteps")}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1 block text-[11px] font-medium uppercase tracking-wide text-neutral-400">
        {label}
      </span>
      {children}
    </label>
  );
}

function toBody(c: Context): ContextBody {
  const clean = (xs: string[]) => xs.map((x) => x.trim()).filter(Boolean);
  return {
    description: c.description,
    goal: c.goal?.trim() || null,
    currentState: c.currentState?.trim() || null,
    decisions: clean(c.decisions),
    tried: clean(c.tried),
    relevantFiles: clean(c.relevantFiles),
    constraints: clean(c.constraints),
    openIssues: clean(c.openIssues),
    nextSteps: clean(c.nextSteps),
  };
}
