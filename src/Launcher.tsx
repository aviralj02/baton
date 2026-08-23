import { useCallback, useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import * as api from "./lib/api";
import { ENTER_LABEL } from "./lib/platform";
import type { ProjectHit } from "./types";

export default function Launcher() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<ProjectHit[]>([]);
  const [selected, setSelected] = useState("");
  const [toast, setToast] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const queryRef = useRef(query);

  const reload = useCallback(async (q: string) => {
    try {
      setRows(q.trim() ? await api.searchProjects(q) : await api.listProjects());
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

  // The agent writes pages while the user is still in a session; without this
  // the launcher shows a stale list until the app restarts.
  useEffect(() => {
    const un = api.onWikiChanged(() => {
      void reload(query);
    });
    return () => void un.then((f) => f());
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
    void api.hideLauncher();
  }, []);

  // The launcher is a single list now, so Escape always closes it.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        dismiss();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dismiss]);

  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 2000);
      return () => clearTimeout(t);
    }
  }, [toast]);

  /// The only action in the launcher: the whole of one project's context on
  /// the clipboard. Every page of it, assembled — the file split is an
  /// organisational detail of the wiki folder, not something a user chooses
  /// between at the moment they want to paste.
  const copyProject = async (slug?: string) => {
    try {
      const copied = await api.copyPrimer(slug);
      setToast(`${copied.project} context copied`);
      setTimeout(dismiss, 350);
    } catch (e) {
      setToast(String(e));
    }
  };

  const trimmed = query.trim();

  return (
    <Shell toast={toast}>
      <Command
        shouldFilter={false} // ranking comes from SQLite bm25, not cmdk
        value={selected}
        onValueChange={setSelected}
        className="flex h-full flex-col"
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
          {/* The primary action, and the first thing selected. Hidden once the
              user types, because then they are hunting one page and Enter must
              belong to the top result rather than to the brief. */}

          {rows.length === 0 && (
            <div className="px-3 py-6 text-center text-sm text-neutral-400">
              {trimmed
                ? `No project matches “${trimmed}”.`
                : "No projects yet."}
              {!trimmed && (
                <button
                  onClick={() => void api.openMainWindow()}
                  className="mt-1 block w-full text-center text-[13px] text-neutral-500 underline-offset-2 hover:underline dark:text-neutral-400"
                >
                  Open Baton to finish setup
                </button>
              )}
            </div>
          )}

          {rows.length > 0 && (
            <Group heading={trimmed ? "Results" : "Projects"}>
              {rows.map((r) => (
                <ProjectItem
                  key={r.slug}
                  hit={r}
                  onSelect={() => void copyProject(r.slug)}
                />
              ))}
            </Group>
          )}

          <Group heading="Actions">
            <Item onSelect={() => void api.openMainWindow()}>Open main window</Item>
          </Group>
        </Command.List>

        <Footer>
          <span>↑↓ Navigate</span>
          <span>{ENTER_LABEL} Copy full context</span>
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
/**
 * One project. The launcher shows nothing smaller: the pages inside are how the
 * wiki organises itself on disk, not a choice worth putting in front of someone
 * who pressed a hotkey to get their context back.
 */
function ProjectItem({ hit, onSelect }: { hit: ProjectHit; onSelect: () => void }) {
  return (
    <Command.Item
      value={`${hit.slug} ${hit.title}`}
      onSelect={onSelect}
      className={`${ITEM_BASE} flex items-center gap-2`}
    >
      <span className="truncate font-medium">{hit.title}</span>
      <span className="ml-auto shrink-0 pl-3 text-xs text-neutral-400">
        {relativeTime(hit.updated)}
      </span>
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
