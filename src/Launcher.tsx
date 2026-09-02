import { useCallback, useEffect, useRef, useState } from "react";
import { Command } from "cmdk";
import * as api from "./lib/api";
import { Logo } from "./components/Logo";
import { IS_MAC } from "./lib/platform";
import { ArrowDownIcon, ArrowUpIcon, ReturnIcon } from "./components/Icon";
import { Key } from "./components/Key";
import { CAPS } from "./components/Label";
import type { ProjectHit } from "./types";

export default function Launcher() {
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<ProjectHit[]>([]);
  const [selected, setSelected] = useState("");
  const [toast, setToast] = useState<string | null>(null);
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
  /// the clipboard. Every page of it, assembled. The file split is an
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
        <div data-tauri-drag-region className="px-3 pt-3 pb-1">
          {/* focus-within rather than an outline: the ring clipped against the panel edge. */}
          <div className="flex items-center gap-2.5 rounded-lg border border-line bg-hover px-3 transition-all duration-200 focus-within:border-brand/45 focus-within:bg-active">
            {/* The mark sits where a magnifier would: branding that is also the affordance. */}
            <Logo size={15} className="shrink-0 text-brand" />
            <Command.Input
              value={query}
              onValueChange={setQuery}
              placeholder="Search projects"
              className="w-full bg-transparent py-2.5 text-input text-ink outline-none placeholder:text-input placeholder:text-muted"
            />
          </div>
        </div>

        <Command.List className="flex-1 overflow-y-auto px-2 pb-2">
          {rows.length === 0 && (
            <div className="px-3 py-10 text-center">
              <p className="font-serif text-title text-body">
                {trimmed ? `Nothing matches “${trimmed}”.` : "No context filed yet."}
              </p>
              {!trimmed && (
                <p className="mt-2 text-ui leading-relaxed text-muted">
                  Run <code className="font-mono text-brand">/baton</code> at the end of a
                  session and your projects appear here.
                </p>
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

          {!trimmed && (
            <Group heading="Actions">
              <Item onSelect={() => void api.openMainWindow()}>Open Baton</Item>
            </Group>
          )}
        </Command.List>

        <Footer>
          <Key label="Up arrow">
            <ArrowUpIcon size={11} />
          </Key>
          <Key label="Down arrow">
            <ArrowDownIcon size={11} />
          </Key>
          <span className="mr-2">Move</span>
          <Key>esc</Key>
          <span>Close</span>
        </Footer>
      </Command>
    </Shell>
  );
}

/**
 * The panel surface.
 *
 * The fill is not optional: vibrancy samples whatever is behind the window, so
 * without it a dark-mode panel over a white background renders light and takes
 * the light text with it. What caused the doubled edge was the 1px border stroke
 * landing on the same 12px arc as the native one, so only that is gone. The
 * radius still has to match `apply_vibrancy` or the corners paint square.
 */
function Shell({ children, toast }: { children: React.ReactNode; toast: string | null }) {
  return (
    <div
      className={`relative h-screen w-screen overflow-hidden rounded-xl bg-float/85 ${
        IS_MAC ? "" : "border border-line"
      }`}
    >
      {children}
      {toast && (
        <div className="pointer-events-none absolute bottom-11 left-1/2 -translate-x-1/2 rounded-full bg-ink px-3.5 py-1.5 text-ui text-surface shadow-lg">
          {toast}
        </div>
      )}
    </div>
  );
}

function Group({ heading, children }: { heading: string; children: React.ReactNode }) {
  return (
    <Command.Group
      heading={<span className={CAPS}>{heading}</span>}
      className="mb-1 [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2"
    >
      {children}
    </Command.Group>
  );
}

// The amber bar is the selection: a pseudo-element that scales in from nothing.
const ITEM_BASE =
  "relative flex h-9 cursor-default items-center gap-2 rounded-md px-3 text-read text-body transition-colors duration-100 before:absolute before:left-0 before:top-1/2 before:h-4 before:w-0.5 before:-translate-y-1/2 before:scale-y-0 before:rounded-full before:bg-brand before:transition-transform before:duration-150 data-[selected=true]:bg-active data-[selected=true]:text-ink data-[selected=true]:before:scale-y-100";

function Item({
  children,
  onSelect,
}: {
  children: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <Command.Item onSelect={onSelect} className={ITEM_BASE}>
      {children}
    </Command.Item>
  );
}

/**
 * One project. The launcher shows nothing smaller: the pages inside are how the
 * wiki organises itself on disk, not a choice worth putting in front of someone
 * who pressed a hotkey to get their context back.
 *
 * Selecting a row turns its page count into the action that will consume it, so
 * the row states both what is there and what Enter does with it.
 */
function ProjectItem({ hit, onSelect }: { hit: ProjectHit; onSelect: () => void }) {
  return (
    <Command.Item
      value={`${hit.slug} ${hit.title}`}
      onSelect={onSelect}
      className={`${ITEM_BASE} group`}
    >
      <span className="truncate font-medium">{hit.title}</span>
      <span className="ml-auto shrink-0 pl-3">
        <span className="tnum font-mono text-meta text-muted group-data-[selected=true]:hidden">
          {hit.pageCount} {hit.pageCount === 1 ? "page" : "pages"}
        </span>
        <span className="hidden items-center gap-1 text-meta font-medium text-brand group-data-[selected=true]:flex">
          <ReturnIcon size={12} />
          Copy {hit.pageCount === 1 ? "page" : "all"}
        </span>
      </span>
    </Command.Item>
  );
}

function Footer({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-1.5 border-t border-line px-4 py-2 text-meta text-muted">
      {children}
    </div>
  );
}
