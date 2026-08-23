import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import * as api from "./lib/api";
import { relativeTime } from "./Launcher";
import { ContextDetail } from "./components/ContextDetail";
import { PageDetail } from "./components/PageDetail";
import { Setup, useSetupGate } from "./components/Setup";
import { PasteConversation } from "./components/PasteConversation";
import { Logo } from "./components/Logo";
import { Dot } from "./components/Dot";
import type { Context, ContextSummary, Page, PageHit } from "./types";

/**
 * Pages are the source of truth and lead the sidebar. Contexts are the old
 * hand-written store, still listed while any exist, because Phase 1's migration
 * out to markdown has not run and they would otherwise be unreachable.
 */
type Selection =
  | { kind: "page"; page: Page; backlinks: PageHit[] }
  | { kind: "context"; context: Context }
  | null;

export default function Browser() {
  const [pages, setPages] = useState<PageHit[]>([]);
  const [contexts, setContexts] = useState<ContextSummary[]>([]);
  const [selected, setSelected] = useState<Selection>(null);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState<string | null>(null);
  const [confirmWipe, setConfirmWipe] = useState(false);
  // A fresh install has no pages and no way to write one; setup is the only
  // useful thing to show until the /baton command exists.
  const setup = useSetupGate();
  const [showSetup, setShowSetup] = useState(false);
  const [pasting, setPasting] = useState<
    null | { kind: "create" } | { kind: "update"; id: string; name: string }
  >(null);

  const reload = useCallback(async (q: string) => {
    try {
      const trimmed = q.trim();
      const [nextPages, nextContexts] = await Promise.all([
        trimmed ? api.searchPages(trimmed) : api.listPages(),
        trimmed ? api.searchContexts(trimmed) : api.listContexts(),
      ]);
      setPages(nextPages);
      setContexts(nextContexts);
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

  const openContext = async (id: string) => {
    try {
      setSelected({ kind: "context", context: await api.getContext(id) });
    } catch (e) {
      setToast(String(e));
    }
  };

  const create = async () => {
    try {
      const context = await api.saveContext({ name: "Untitled context" });
      await reload(query);
      setSelected({ kind: "context", context });
    } catch (e) {
      setToast(String(e));
    }
  };

  const selectedId =
    selected?.kind === "page"
      ? selected.page.id
      : selected?.kind === "context"
        ? selected.context.id
        : null;

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
        <button
          onClick={() => void create()}
          className="cursor-pointer rounded-md px-2.5 py-1 text-xs transition-all duration-150 hover:bg-black/5 active:scale-[0.98] dark:text-neutral-300 dark:hover:bg-white/10"
        >
          + Empty
        </button>
        <button
          onClick={() => setPasting({ kind: "create" })}
          className="cursor-pointer rounded-md bg-neutral-900 px-2.5 py-1 text-xs font-medium text-white transition-all duration-150 hover:bg-neutral-700 active:scale-[0.98] dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
        >
          Paste conversation
        </button>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside className="w-60 shrink-0 overflow-y-auto border-r border-black/10 p-2 dark:border-white/10">
          {pages.length === 0 && contexts.length === 0 && (
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

          {contexts.length > 0 && <SidebarHeading>Contexts</SidebarHeading>}
          {contexts.map((row) => (
            <SidebarRow
              key={row.id}
              active={selectedId === row.id}
              onClick={() => void openContext(row.id)}
              title={row.name}
              meta={relativeTime(row.updatedAt)}
              faded={false}
            />
          ))}

          <div className="mt-4 border-t border-black/10 pt-3 dark:border-white/10">
            {confirmWipe ? (
              <div className="px-1">
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400">
                  Delete every context and clear the search index? The markdown files in
                  ~/Baton are not touched, and the index rebuilds from them.
                </p>
                <div className="mt-2 flex gap-2">
                  <button
                    onClick={async () => {
                      try {
                        await api.deleteAllData();
                        setSelected(null);
                        setConfirmWipe(false);
                        await api.syncWiki();
                        await reload(query);
                        setToast("Local data deleted");
                      } catch (e) {
                        setToast(String(e));
                      }
                    }}
                    className="cursor-pointer rounded bg-red-600 px-2 py-1 text-[11px] font-medium text-white transition-all duration-150 hover:bg-red-700 active:scale-[0.98]"
                  >
                    Delete all
                  </button>
                  <button
                    onClick={() => setConfirmWipe(false)}
                    className="cursor-pointer px-1 text-[11px] text-neutral-500 transition-all duration-150 hover:underline"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              // PRD §9 requires a discoverable delete-everything action.
              <button
                onClick={() => setConfirmWipe(true)}
                className="cursor-pointer px-1.5 text-[11px] text-neutral-400 transition-all duration-150 hover:text-red-500"
              >
                Delete local data…
              </button>
            )}
          </div>
        </aside>

        <main className="min-w-0 flex-1">
          {pasting ? (
            <PasteConversation
              mode={pasting}
              onDone={async (context) => {
                setPasting(null);
                setSelected({ kind: "context", context });
                await reload(query);
                setToast("Context generated");
              }}
              onCancel={() => setPasting(null)}
              onError={setToast}
            />
          ) : selected?.kind === "page" ? (
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
          ) : selected?.kind === "context" ? (
            <ContextDetail
              context={selected.context}
              onBack={() => setSelected(null)}
              onUpdate={() =>
                setPasting({
                  kind: "update",
                  id: selected.context.id,
                  name: selected.context.name,
                })
              }
              onHandoff={async () => {
                try {
                  setToast("Writing handoff…");
                  await api.generateHandoff(selected.context.id);
                  setToast("Handoff copied");
                } catch (e) {
                  setToast(String(e));
                }
              }}
              onCopy={async () => {
                try {
                  await api.copyContext(selected.context.id);
                  setToast("Copied to clipboard");
                } catch (e) {
                  setToast(String(e));
                }
              }}
              onSaved={async (context) => {
                setSelected({ kind: "context", context });
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
