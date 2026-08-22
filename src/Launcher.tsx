import { useCallback, useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import { openPath } from "@tauri-apps/plugin-opener";
import * as api from "./lib/api";
import { MOD_LABEL, ENTER_LABEL, hasMod } from "./lib/platform";
import type { Context, PageHit } from "./types";
import { ContextDetail } from "./components/ContextDetail";
import { PasteConversation } from "./components/PasteConversation";

type Mode =
  | { kind: "list" }
  | { kind: "detail"; id: string }
  | { kind: "paste"; name: string }
  | { kind: "update"; id: string; name: string };

export default function Launcher() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<PageHit[]>([]);
  const [selected, setSelected] = useState("");
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  const [detail, setDetail] = useState<Context | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const queryRef = useRef(query);

  const reload = useCallback(async (q: string) => {
    try {
      setRows(q.trim() ? await api.searchPages(q) : await api.listPages());
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  // Search runs in SQLite (FTS5), not in the client, so every keystroke is a
  // round trip. Debounced to keep typing smooth on large stores.
  useEffect(() => {
    const t = setTimeout(() => void reload(query), 80);
    return () => clearTimeout(t);
  }, [query, reload]);

  useEffect(() => {
    queryRef.current = query;
  }, [query]);

  // The wiki is edited outside this app, by an agent running `/baton` in a
  // terminal or by hand in an editor. Re-sweeping on every summon is what keeps
  // the index from lagging behind the files. The sweep skips anything whose
  // mtime and size are unchanged, so the common case costs almost nothing.
  useEffect(() => {
    const unlisten = api.onLauncherShown(() => {
      void (async () => {
        try {
          await api.syncWiki();
        } catch (e) {
          setToast(String(e));
        }
        await reload(queryRef.current);
      })();
    });
    return () => void unlisten.then((off) => off());
  }, [reload]);

  const dismiss = useCallback(() => {
    setQuery("");
    setMode({ kind: "list" });
    setDetail(null);
    void api.hideLauncher();
  }, []);

  // Escape steps back one level rather than always closing — inside a detail
  // view, closing the whole launcher would lose the user's place.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (mode.kind === "list") dismiss();
        else {
          setMode({ kind: "list" });
          setDetail(null);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, dismiss]);

  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 2000);
      return () => clearTimeout(t);
    }
  }, [toast]);

  const copyPage = async (id: string) => {
    try {
      await api.copyPage(id);
      setToast("Copied to clipboard");
      // The whole point is pasting elsewhere, so get out of the way.
      setTimeout(dismiss, 350);
    } catch (e) {
      setToast(String(e));
    }
  };

  /// Hand the file to whatever owns .md on this machine, which is the editor
  /// the user already writes these pages in.
  const openFile = async (hit: PageHit) => {
    try {
      await openPath(hit.path);
      dismiss();
    } catch (e) {
      setToast(String(e));
    }
  };

  const copyContext = async (id: string) => {
    try {
      await api.copyContext(id);
      setToast("Copied to clipboard");
      setTimeout(dismiss, 350);
    } catch (e) {
      setToast(String(e));
    }
  };

  /// Manual creation, kept for a context you want to write yourself.
  const createBlank = async (name: string) => {
    try {
      const ctx = await api.saveContext({ name });
      setQuery("");
      await reload("");
      setDetail(ctx);
      setMode({ kind: "detail", id: ctx.id });
    } catch (e) {
      setToast(String(e));
    }
  };

  const afterGenerate = async (ctx: Context) => {
    setQuery("");
    await reload("");
    setDetail(ctx);
    setMode({ kind: "detail", id: ctx.id });
    setToast("Context generated");
  };

  if (mode.kind === "paste" || mode.kind === "update") {
    return (
      <Shell toast={toast}>
        <PasteConversation
          mode={
            mode.kind === "paste"
              ? { kind: "create" }
              : { kind: "update", id: mode.id, name: mode.name }
          }
          initialName={mode.kind === "paste" ? mode.name : undefined}
          onDone={afterGenerate}
          onCancel={() => setMode({ kind: "list" })}
          onError={setToast}
        />
      </Shell>
    );
  }

  if (mode.kind === "detail" && detail) {
    return (
      <Shell toast={toast}>
        <ContextDetail
          context={detail}
          onBack={() => {
            setMode({ kind: "list" });
            setDetail(null);
          }}
          onCopy={() => copyContext(detail.id)}
          onUpdate={() =>
            setMode({ kind: "update", id: detail.id, name: detail.name })
          }
          onHandoff={async () => {
            try {
              setToast("Writing handoff…");
              await api.generateHandoff(detail.id);
              setToast("Handoff copied");
              setTimeout(dismiss, 350);
            } catch (e) {
              setToast(String(e));
            }
          }}
          onSaved={async (c) => {
            setDetail(c);
            await reload(query);
            setToast("Saved");
          }}
          onDeleted={async () => {
            setMode({ kind: "list" });
            setDetail(null);
            await reload(query);
            setToast("Deleted");
          }}
          onError={setToast}
        />
      </Shell>
    );
  }

  const trimmed = query.trim();

  return (
    <Shell toast={toast}>
      <Command
        shouldFilter={false} // ranking comes from SQLite bm25, not cmdk
        value={selected}
        onValueChange={setSelected}
        className="flex h-full flex-col"
        onKeyDown={(e) => {
          if (hasMod(e) && e.key === "Enter") {
            e.preventDefault();
            // cmdk normalises the value it reports, so match case-insensitively.
            // Rows that are not pages match nothing and fall through.
            const hit = rows.find((r) => r.id.toLowerCase() === selected.toLowerCase());
            if (hit) void openFile(hit);
          }
        }}
      >
        <div data-tauri-drag-region className="border-b border-black/5 dark:border-white/5">
          <Command.Input
            ref={inputRef}
            autoFocus
            value={query}
            onValueChange={setQuery}
            placeholder="Search your wiki..."
            className="w-full bg-transparent px-4 py-3.5 text-[15px] outline-none placeholder:text-[15px] placeholder:text-neutral-400 dark:text-neutral-100"
          />
        </div>

        <Command.List className="flex-1 overflow-y-auto p-2">
          {rows.length === 0 && (
            <div className="px-3 py-6 text-center text-sm text-neutral-400">
              {trimmed
                ? `Nothing matches “${trimmed}”.`
                : "No pages yet. Run /baton at the end of a session to write one."}
            </div>
          )}

          {rows.length > 0 && (
            <Group heading={trimmed ? "Results" : "Recent"}>
              {rows.map((r) => (
                <PageItem key={r.id} hit={r} onSelect={() => void copyPage(r.id)} />
              ))}
            </Group>
          )}

          <Group heading="Create">
            <Item onSelect={() => setMode({ kind: "paste", name: trimmed })}>
              Paste a conversation
              {trimmed && <span className="text-neutral-400"> as “{trimmed}”</span>}
            </Item>
            {trimmed && (
              <Item onSelect={() => void createBlank(trimmed)}>
                Create empty context{" "}
                <span className="text-neutral-400">“{trimmed}”</span>
              </Item>
            )}
          </Group>

          <Group heading="Actions">
            <Item onSelect={() => void api.openMainWindow()}>Open main window</Item>
          </Group>
        </Command.List>

        <Footer>
          <span>↑↓ Navigate</span>
          <span>{ENTER_LABEL} Copy page</span>
          <span className="ml-auto">
            {MOD_LABEL}
            {ENTER_LABEL} Open file
          </span>
        </Footer>
      </Command>
    </Shell>
  );
}

function Shell({ children, toast }: { children: React.ReactNode; toast: string | null }) {
  return (
    <div className="relative h-screen w-screen overflow-hidden rounded-xl border border-black/10 bg-white/60 dark:border-white/10 dark:bg-neutral-900/50">
      {children}
      {toast && (
        <div className="pointer-events-none absolute bottom-10 left-1/2 -translate-x-1/2 rounded-md bg-neutral-900/90 px-3 py-1.5 text-xs text-white shadow-lg dark:bg-white/90 dark:text-neutral-900">
          {toast}
        </div>
      )}
    </div>
  );
}

export function Group({ heading, children }: { heading: string; children: React.ReactNode }) {
  return (
    <Command.Group
      heading={heading}
      className="[&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-neutral-400"
    >
      {children}
    </Command.Group>
  );
}

const ITEM_BASE =
  "cursor-default rounded-md px-3 py-2 text-sm text-neutral-700 data-[selected=true]:bg-black/5 dark:text-neutral-200 dark:data-[selected=true]:bg-white/10";

export function Item({
  children,
  onSelect,
}: {
  children: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <Command.Item onSelect={onSelect} className={`${ITEM_BASE} flex items-center gap-2`}>
      {children}
    </Command.Item>
  );
}

/**
 * A wiki page row. Two lines when the hit carries a snippet, one when it does
 * not, so the browse list stays dense and a search result shows why it matched.
 */
function PageItem({ hit, onSelect }: { hit: PageHit; onSelect: () => void }) {
  return (
    <Command.Item
      value={hit.id}
      onSelect={onSelect}
      className={`${ITEM_BASE} flex flex-col items-stretch gap-0.5`}
    >
      <div className="flex items-center gap-2">
        <span className="truncate">{hit.title || hit.id}</span>
        {/* The wiki keeps dead pages on purpose, so say which ones are dead. */}
        {hit.status !== "current" && (
          <span className="shrink-0 rounded bg-black/5 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-neutral-500 dark:bg-white/10 dark:text-neutral-400">
            {hit.status}
          </span>
        )}
        <span className="ml-auto shrink-0 pl-3 text-xs text-neutral-400">{hit.type}</span>
        <span className="shrink-0 text-xs text-neutral-400">{relativeTime(hit.updated)}</span>
      </div>
      {hit.snippet && (
        <span className="truncate text-xs text-neutral-400">{hit.snippet}</span>
      )}
    </Command.Item>
  );
}

export function Footer({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-4 border-t border-black/5 px-4 py-2 text-[11px] text-neutral-400 dark:border-white/5">
      {children}
    </div>
  );
}

/** Compact relative time for list rows; falls back to the raw value. */
export function relativeTime(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "";
  const mins = Math.floor((Date.now() - then) / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(iso).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
