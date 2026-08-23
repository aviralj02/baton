import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import * as api from "./lib/api";
import { relativeTime } from "./Launcher";
import { PageDetail } from "./components/PageDetail";
import { Setup, useSetupGate } from "./components/Setup";
import { Logo } from "./components/Logo";
import { Dot } from "./components/Dot";
import type { Page, PageHit } from "./types";

/** Markdown files under ~/Baton are the only store; the index is derived. */
type Selection = { kind: "page"; page: Page; backlinks: PageHit[] } | null;

export default function Browser() {
  const [pages, setPages] = useState<PageHit[]>([]);
  const [selected, setSelected] = useState<Selection>(null);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState<string | null>(null);
  // A fresh install has no pages and no way to write one; setup is the only
  // useful thing to show until the /baton command exists.
  const setup = useSetupGate();
  const [showSetup, setShowSetup] = useState(false);

  const reload = useCallback(async (q: string) => {
    try {
      const trimmed = q.trim();
      setPages(trimmed ? await api.searchPages(trimmed) : await api.listPages());
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  useEffect(() => {
    const t = setTimeout(() => void reload(query), 80);
    return () => clearTimeout(t);
  }, [query, reload]);

  // Reflect edits made in an external editor without a restart.
  useEffect(() => {
    const un = api.onWikiChanged(() => void reload(query));
    return () => void un.then((f) => f());
  }, [query, reload]);

  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 2000);
      return () => clearTimeout(t);
    }
  }, [toast]);

  const openPage = useCallback(async (id: string) => {
    try {
      const [page, backlinks] = await Promise.all([
        api.readPage(id),
        api.pageBacklinks(id),
      ]);
      setSelected({ kind: "page", page, backlinks });
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  const selectedId = selected?.kind === "page" ? selected.page.id : null;

  if ((setup.needed || showSetup) && setup.status) {
    return (
      <div className="flex h-screen w-screen flex-col bg-white text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
        <header
          data-tauri-drag-region
          className="h-11 shrink-0 border-b border-black/10 dark:border-white/10"
        />
        <Setup
          status={setup.status}
          onDone={() => {
            setShowSetup(false);
            setup.dismiss();
            setup.refresh();
            void reload(query);
          }}
        />
      </div>
    );
  }

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
          className="ml-4 w-64 rounded-md border border-black/10 bg-black/3 px-2.5 py-1 text-sm outline-none placeholder:text-sm transition-all duration-200 focus:border-black/25 dark:border-white/10 dark:bg-white/5"
        />
        <button
          onClick={() => void api.syncWiki().then(() => reload(query))}
          className="ml-auto cursor-pointer rounded-md px-2.5 py-1 text-xs transition-all duration-150 hover:bg-black/5 active:scale-[0.98] dark:text-neutral-300 dark:hover:bg-white/10"
        >
          Refresh
        </button>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside className="w-60 shrink-0 overflow-y-auto border-r border-black/10 p-2 dark:border-white/10">
          {pages.length === 0 && (
            <p className="px-2 py-4 text-xs text-neutral-400">
              {query.trim()
                ? "No matches."
                : "No pages yet. Run /baton at the end of a session to write one."}
            </p>
          )}

          {pages.length > 0 && <SidebarHeading>Pages</SidebarHeading>}
          {pages.map((hit) => (
            <SidebarRow
              key={hit.id}
              active={selectedId === hit.id}
              onClick={() => void openPage(hit.id)}
              title={hit.title || hit.id}
              meta={
                <span className="flex items-center gap-1.5">
                  {hit.type}
                  <Dot />
                  {relativeTime(hit.updated)}
                </span>
              }
              faded={hit.status !== "current"}
            />
          ))}

        </aside>

        <main className="min-w-0 flex-1">
          {selected?.kind === "page" ? (
            <PageDetail
              page={selected.page}
              backlinks={selected.backlinks}
              onOpenPage={(id) => void openPage(id)}
              onCopy={async () => {
                try {
                  await api.copyPage(selected.page.id);
                  setToast("Copied to clipboard");
                } catch (e) {
                  setToast(String(e));
                }
              }}
              onOpenFile={async () => {
                try {
                  await openPath(selected.page.path);
                } catch (e) {
                  setToast(String(e));
                }
              }}
            />
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-neutral-400">
              <Logo size={40} className="opacity-25" />
              <p className="text-sm">
                Select a page, or press ⌘⇧Space anywhere to summon the launcher.
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

function SidebarHeading({ children }: { children: React.ReactNode }) {
  return (
    <p className="px-2.5 pb-1 pt-3 text-[10px] font-medium uppercase tracking-wide text-neutral-400">
      {children}
    </p>
  );
}

function SidebarRow({
  active,
  onClick,
  title,
  meta,
  faded,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  meta: React.ReactNode;
  faded: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`mb-0.5 block w-full cursor-pointer rounded-md px-2.5 py-1.5 text-left transition-all duration-150 active:scale-[0.98] ${
        active
          ? "bg-black/7 dark:bg-white/10"
          : "hover:bg-black/4 dark:hover:bg-white/5"
      }`}
    >
      <span className={`block truncate text-sm ${faded ? "text-neutral-400" : ""}`}>
        {title}
      </span>
      <span className="block truncate text-[11px] text-neutral-400">{meta}</span>
    </button>
  );
}
