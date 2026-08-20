import { useCallback, useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import * as api from "./lib/api";
import { MOD_LABEL, ENTER_LABEL, hasMod } from "./lib/platform";
import type { Context, ContextSummary } from "./types";
import { ContextDetail } from "./components/ContextDetail";

type Mode = { kind: "list" } | { kind: "detail"; id: string } | { kind: "create" };

export default function Launcher() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<ContextSummary[]>([]);
  const [mode, setMode] = useState<Mode>({ kind: "list" });
  const [detail, setDetail] = useState<Context | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const reload = useCallback(async (q: string) => {
    try {
      setRows(q.trim() ? await api.searchContexts(q) : await api.listContexts());
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

  const open = async (id: string) => {
    try {
      setDetail(await api.getContext(id));
      setMode({ kind: "detail", id });
    } catch (e) {
      setToast(String(e));
    }
  };

  const copy = async (id: string) => {
    try {
      await api.copyContext(id);
      setToast("Copied to clipboard");
      // The whole point is pasting elsewhere, so get out of the way.
      setTimeout(dismiss, 350);
    } catch (e) {
      setToast(String(e));
    }
  };

  const create = async (name: string) => {
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

  if (mode.kind === "detail" && detail) {
    return (
      <Shell toast={toast}>
        <ContextDetail
          context={detail}
          onBack={() => {
            setMode({ kind: "list" });
            setDetail(null);
          }}
          onCopy={() => copy(detail.id)}
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
  const exactExists = rows.some((r) => r.name.toLowerCase() === trimmed.toLowerCase());

  return (
    <Shell toast={toast}>
      <Command
        shouldFilter={false} // ranking comes from SQLite bm25, not cmdk
        className="flex h-full flex-col"
        onKeyDown={(e) => {
          if (hasMod(e) && e.key === "Enter") {
            e.preventDefault();
            const id = rows[0]?.id;
            if (id) void copy(id);
          }
        }}
      >
        <div data-tauri-drag-region className="border-b border-black/5 dark:border-white/5">
          <Command.Input
            ref={inputRef}
            autoFocus
            value={query}
            onValueChange={setQuery}
            placeholder="Search or create context..."
            className="w-full bg-transparent px-4 py-3.5 text-[15px] outline-none placeholder:text-neutral-400 dark:text-neutral-100"
          />
        </div>

        <Command.List className="flex-1 overflow-y-auto p-2">
          {rows.length === 0 && !trimmed && (
            <div className="px-3 py-6 text-center text-sm text-neutral-400">
              No contexts yet. Type a name to create one.
            </div>
          )}

          {trimmed && !exactExists && (
            <Group heading="Create">
              <Item onSelect={() => void create(trimmed)}>
                Create context <span className="text-neutral-400">“{trimmed}”</span>
              </Item>
            </Group>
          )}

          {rows.length > 0 && (
            <Group heading={trimmed ? "Results" : "Recent"}>
              {rows.map((r) => (
                <Item key={r.id} onSelect={() => void open(r.id)}>
                  <span className="truncate">{r.name}</span>
                  <span className="ml-auto shrink-0 pl-3 text-xs text-neutral-400">
                    {relativeTime(r.updatedAt)}
                  </span>
                </Item>
              ))}
            </Group>
          )}

          <Group heading="Actions">
            <Item onSelect={() => void api.openMainWindow()}>Open main window</Item>
          </Group>
        </Command.List>

        <Footer>
          <span>↑↓ Navigate</span>
          <span>{ENTER_LABEL} Open</span>
          <span className="ml-auto">
            {MOD_LABEL}
            {ENTER_LABEL} Copy top result
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

export function Item({
  children,
  onSelect,
}: {
  children: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <Command.Item
      onSelect={onSelect}
      className="flex cursor-default items-center gap-2 rounded-md px-3 py-2 text-sm text-neutral-700 data-[selected=true]:bg-black/5 dark:text-neutral-200 dark:data-[selected=true]:bg-white/10"
    >
      {children}
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
