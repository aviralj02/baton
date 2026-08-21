import { useCallback, useEffect, useState } from "react";
import * as api from "./lib/api";
import { relativeTime } from "./Launcher";
import { ContextDetail } from "./components/ContextDetail";
import { PasteConversation } from "./components/PasteConversation";
import { Logo } from "./components/Logo";
import type { Context, ContextSummary } from "./types";

export default function Browser() {
  const [rows, setRows] = useState<ContextSummary[]>([]);
  const [selected, setSelected] = useState<Context | null>(null);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState<string | null>(null);
  const [confirmWipe, setConfirmWipe] = useState(false);
  const [pasting, setPasting] = useState<null | { kind: "create" } | { kind: "update"; id: string; name: string }>(null);

  const reload = useCallback(async (q: string) => {
    try {
      setRows(q.trim() ? await api.searchContexts(q) : await api.listContexts());
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  useEffect(() => {
    const t = setTimeout(() => void reload(query), 80);
    return () => clearTimeout(t);
  }, [query, reload]);

  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 2000);
      return () => clearTimeout(t);
    }
  }, [toast]);

  const select = async (id: string) => {
    try {
      setSelected(await api.getContext(id));
    } catch (e) {
      setToast(String(e));
    }
  };

  const create = async () => {
    try {
      const ctx = await api.saveContext({ name: "Untitled context" });
      await reload(query);
      setSelected(ctx);
    } catch (e) {
      setToast(String(e));
    }
  };

  return (
    <div className="flex h-screen w-screen flex-col bg-white text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
      <header
        data-tauri-drag-region
        className="flex items-center gap-3 border-b border-black/10 px-4 py-2.5 dark:border-white/10"
      >
        {/* pl-16 clears the macOS traffic lights on a decorated window. */}
        <span className="flex items-center gap-2 text-sm font-medium">
          <Logo size={17} />
          Baton
        </span>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search…"
          className="ml-4 w-64 rounded-md border border-black/10 bg-black/[0.03] px-2.5 py-1 text-sm outline-none focus:border-black/25 dark:border-white/10 dark:bg-white/5"
        />
        <button
          onClick={() => void create()}
          className="ml-auto rounded-md px-2.5 py-1 text-xs hover:bg-black/5 dark:text-neutral-300 dark:hover:bg-white/10"
        >
          + Empty
        </button>
        <button
          onClick={() => setPasting({ kind: "create" })}
          className="rounded-md bg-neutral-900 px-2.5 py-1 text-xs font-medium text-white hover:bg-neutral-700 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
        >
          Paste conversation
        </button>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside className="w-60 shrink-0 overflow-y-auto border-r border-black/10 p-2 dark:border-white/10">
          {rows.length === 0 && (
            <p className="px-2 py-4 text-xs text-neutral-400">
              {query.trim() ? "No matches." : "No contexts yet."}
            </p>
          )}
          {rows.map((r) => (
            <button
              key={r.id}
              onClick={() => void select(r.id)}
              className={`mb-0.5 block w-full rounded-md px-2.5 py-1.5 text-left ${
                selected?.id === r.id
                  ? "bg-black/[0.07] dark:bg-white/10"
                  : "hover:bg-black/[0.04] dark:hover:bg-white/5"
              }`}
            >
              <span className="block truncate text-sm">{r.name}</span>
              <span className="block text-[11px] text-neutral-400">
                {relativeTime(r.updatedAt)}
              </span>
            </button>
          ))}

          <div className="mt-4 border-t border-black/10 pt-3 dark:border-white/10">
            {confirmWipe ? (
              <div className="px-1">
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400">
                  Delete every context permanently?
                </p>
                <div className="mt-2 flex gap-2">
                  <button
                    onClick={async () => {
                      try {
                        await api.deleteAllData();
                        setSelected(null);
                        setConfirmWipe(false);
                        await reload(query);
                        setToast("All data deleted");
                      } catch (e) {
                        setToast(String(e));
                      }
                    }}
                    className="rounded bg-red-600 px-2 py-1 text-[11px] font-medium text-white hover:bg-red-700"
                  >
                    Delete all
                  </button>
                  <button
                    onClick={() => setConfirmWipe(false)}
                    className="px-1 text-[11px] text-neutral-500 hover:underline"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              // PRD §9 requires a discoverable delete-everything action.
              <button
                onClick={() => setConfirmWipe(true)}
                className="px-1.5 text-[11px] text-neutral-400 hover:text-red-500"
              >
                Delete all data…
              </button>
            )}
          </div>
        </aside>

        <main className="min-w-0 flex-1">
          {pasting ? (
            <PasteConversation
              mode={pasting}
              onDone={async (c) => {
                setPasting(null);
                setSelected(c);
                await reload(query);
                setToast("Context generated");
              }}
              onCancel={() => setPasting(null)}
              onError={setToast}
            />
          ) : selected ? (
            <ContextDetail
              context={selected}
              onBack={() => setSelected(null)}
              onUpdate={() =>
                setPasting({ kind: "update", id: selected.id, name: selected.name })
              }
              onHandoff={async () => {
                try {
                  setToast("Writing handoff…");
                  await api.generateHandoff(selected.id);
                  setToast("Handoff copied");
                } catch (e) {
                  setToast(String(e));
                }
              }}
              onCopy={async () => {
                try {
                  await api.copyContext(selected.id);
                  setToast("Copied to clipboard");
                } catch (e) {
                  setToast(String(e));
                }
              }}
              onSaved={async (c) => {
                setSelected(c);
                await reload(query);
                setToast("Saved");
              }}
              onDeleted={async () => {
                setSelected(null);
                await reload(query);
                setToast("Deleted");
              }}
              onError={setToast}
            />
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-neutral-400">
              <Logo size={40} className="opacity-25" />
              <p className="text-sm">
                Select a context, or press ⌘⇧Space anywhere to summon the launcher.
              </p>
            </div>
          )}
        </main>
      </div>

      {toast && (
        <div className="pointer-events-none fixed bottom-6 left-1/2 -translate-x-1/2 rounded-md bg-neutral-900/90 px-3 py-1.5 text-xs text-white shadow-lg dark:bg-white/90 dark:text-neutral-900">
          {toast}
        </div>
      )}
    </div>
  );
}
