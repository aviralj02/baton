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

  const [pending, setPending] = useState<"rebuild" | "wipe" | null>(null);
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

          {/* Grouped by project, not one flat list. A flat list of every page
              across every project reads as noise the moment there is more than
              one project, and hides the fact that a project is the unit. */}
          {groupByProject(pages).map(({ slug, name, pages: group }) => (
            <ProjectGroup
              key={name}
              name={name}
              count={group.length}
              // A search should not require reopening every folder to see hits.
              defaultOpen={Boolean(query.trim()) || group.some((h) => h.id === selectedId)}
              onDelete={
                slug
                  ? async () => {
                      try {
                        // The open page may have been inside the project.
                        if (group.some((h) => h.id === selectedId)) setSelected(null);
                        await api.deleteProject(slug);
                        await reload(query);
                        setToast(`${name} moved to Trash`);
                      } catch (e) {
                        setToast(String(e));
                      }
                    }
                  : undefined
              }
            >
              {group.map((hit) => (
                <SidebarRow
                  key={hit.id}
                  active={selectedId === hit.id}
                  onClick={() => void openPage(hit.id)}
                  title={hit.title || readableId(hit.id)}
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
            </ProjectGroup>
          ))}

          <div className="mt-4 border-t border-black/10 pt-3 dark:border-white/10">
            {pending === "rebuild" && (
              <div className="px-1">
                <p className="text-[11px] leading-relaxed text-neutral-500 dark:text-neutral-400">
                  Rebuild the search index from the files in ~/Baton? Nothing is
                  deleted — the markdown is the source of truth and the index is
                  derived from it.
                </p>
                <div className="mt-2 flex gap-2">
                  <button
                    onClick={async () => {
                      try {
                        setSelected(null);
                        setPending(null);
                        // Actually drops the index and re-reads every file. The
                        // previous version only re-swept and still reported a
                        // deletion, which was a false claim about a privacy
                        // action.
                        const report = await api.rebuildIndex();
                        await reload(query);
                        setToast(`Index rebuilt from ${report.indexed} pages`);
                      } catch (e) {
                        setToast(String(e));
                      }
                    }}
                    className="cursor-pointer rounded bg-neutral-900 px-2 py-1 text-[11px] font-medium text-white transition-all duration-150 hover:bg-neutral-700 active:scale-[0.98] dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
                  >
                    Rebuild
                  </button>
                  <button
                    onClick={() => setPending(null)}
                    className="cursor-pointer px-1 text-[11px] text-neutral-500 transition-all duration-150 hover:underline"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}

            {pending === "wipe" && (
              <div className="px-1">
                <p className="text-[11px] leading-relaxed text-neutral-500 dark:text-neutral-400">
                  Move every project and constraint in ~/Baton to the Trash?
                  {pages.length > 0 && ` That is ${pages.length} page${pages.length === 1 ? "" : "s"}.`}{" "}
                  Your schema stays, and everything else can be put back from the
                  Trash.
                </p>
                <ConfirmRow
                  label="Move all to Trash"
                  onConfirm={async () => {
                    try {
                      setSelected(null);
                      setPending(null);
                      await api.deleteEverything();
                      await reload(query);
                      setToast("Everything moved to Trash");
                    } catch (e) {
                      setToast(String(e));
                    }
                  }}
                  onCancel={() => setPending(null)}
                />
              </div>
            )}

            {pending === null && (
              <div className="flex items-center gap-3 px-1.5">
                <button
                  onClick={() => setPending("rebuild")}
                  className="cursor-pointer text-[11px] text-neutral-400 transition-all duration-150 hover:text-neutral-600 dark:hover:text-neutral-200"
                >
                  Rebuild index…
                </button>
                {pages.length > 0 && (
                  <button
                    onClick={() => setPending("wipe")}
                    className="cursor-pointer text-[11px] text-neutral-400 transition-all duration-150 hover:text-red-600 dark:hover:text-red-400"
                  >
                    Delete all…
                  </button>
                )}
              </div>
            )}
          </div>
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
              onDelete={async () => {
                const title = selected.page.title || readableId(selected.page.id);
                try {
                  setSelected(null);
                  await api.deletePage(selected.page.id);
                  await reload(query);
                  setToast(`${title} moved to Trash`);
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

/**
 * A readable stand-in for a page with no title: its last path segment with the
 * hyphens opened out. Pages should carry a `#` heading — lint flags the ones
 * that do not — but a path in the sidebar is worse than an imperfect name.
 */
function readableId(id: string): string {
  const last = id.split("/").pop() ?? id;
  const words = last.replace(/[-_]/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/**
 * The slug is carried alongside the display name because deleting a project
 * needs the folder name, and "Constraints" is a heading rather than a folder —
 * a null slug is what marks a group that cannot be deleted as a unit.
 */
type Group = { slug: string | null; name: string; pages: PageHit[] };

function groupByProject(pages: PageHit[]): Group[] {
  const groups = new Map<string, PageHit[]>();
  for (const hit of pages) {
    // The tilde sorts them after every real project without a special case.
    const key = hit.project ?? "~Constraints";
    const list = groups.get(key) ?? [];
    list.push(hit);
    groups.set(key, list);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, pages]) =>
      key === "~Constraints"
        ? { slug: null, name: "Constraints", pages }
        : { slug: key, name: key, pages },
    );
}

function ProjectGroup({
  name,
  count,
  defaultOpen,
  onDelete,
  children,
}: {
  name: string;
  count: number;
  defaultOpen: boolean;
  /** Absent for Constraints, which is a heading over concepts/ rather than a folder. */
  onDelete?: () => Promise<void>;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const [confirming, setConfirming] = useState(false);

  // A search result that lands in a closed group would be invisible.
  useEffect(() => {
    if (defaultOpen) setOpen(true);
  }, [defaultOpen]);

  if (confirming) {
    return (
      <section className="mb-1 px-2 py-1">
        <p className="text-[11px] leading-relaxed text-neutral-500 dark:text-neutral-400">
          Move <span className="font-medium">{name}</span> and its {count}{" "}
          {count === 1 ? "page" : "pages"} to the Trash?
        </p>
        <ConfirmRow
          label="Move to Trash"
          onConfirm={async () => {
            setConfirming(false);
            await onDelete?.();
          }}
          onCancel={() => setConfirming(false)}
        />
      </section>
    );
  }

  return (
    <section className="mb-1">
      {/* A row, not a button, because the delete control lives inside it and a
          button cannot nest inside a button. */}
      <div className="group flex items-center rounded pr-1 transition-colors hover:bg-black/[0.04] dark:hover:bg-white/5">
        <button
          onClick={() => setOpen((v) => !v)}
          className="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-neutral-400"
        >
          <span className={`transition-transform ${open ? "rotate-90" : ""}`}>›</span>
          <span className="truncate">{name}</span>
          <span className="ml-auto font-normal normal-case">{count}</span>
        </button>
        {onDelete && (
          // Hidden until hover: this sits next to a control used constantly,
          // and a delete that is always visible eventually gets hit.
          <button
            onClick={() => setConfirming(true)}
            title={`Delete ${name}`}
            className="ml-1 cursor-pointer rounded px-1 text-[11px] text-neutral-300 opacity-0 transition-all duration-150 group-hover:opacity-100 hover:text-red-600 dark:text-neutral-600 dark:hover:text-red-400"
          >
            ✕
          </button>
        )}
      </div>
      {open && <div className="ml-1.5 border-l border-black/5 pl-1.5 dark:border-white/5">{children}</div>}
    </section>
  );
}

/** The confirm/cancel pair every destructive action in this window uses. */
function ConfirmRow({
  label,
  onConfirm,
  onCancel,
}: {
  label: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="mt-2 flex gap-2">
      <button
        onClick={onConfirm}
        className="cursor-pointer rounded bg-red-600 px-2 py-1 text-[11px] font-medium text-white transition-all duration-150 hover:bg-red-700 active:scale-[0.98]"
      >
        {label}
      </button>
      <button
        onClick={onCancel}
        className="cursor-pointer px-1 text-[11px] text-neutral-500 transition-all duration-150 hover:underline"
      >
        Cancel
      </button>
    </div>
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
