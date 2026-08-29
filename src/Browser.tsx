import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import * as api from "./lib/api";
import { relativeTime } from "./lib/time";
import { PageDetail } from "./components/PageDetail";
import { Setup, useSetupGate } from "./components/Setup";
import { Settings, prettify } from "./components/Settings";
import { Logo } from "./components/Logo";
import { Tooltip } from "./components/Tooltip";
import { SUMMON_LABEL } from "./lib/platform";
import {
  ChevronIcon,
  InstallIcon,
  RefreshIcon,
  SearchIcon,
  SettingsIcon,
  TrashIcon,
  TypeIcon,
  TYPE_LABEL,
} from "./components/Icon";
import type { Page, PageHit } from "./types";

/** Markdown files under ~/Baton are the only store; the index is derived. */
type Selection = { kind: "page"; page: Page; backlinks: PageHit[] } | null;

export default function Browser() {
  const [pages, setPages] = useState<PageHit[]>([]);
  const [selected, setSelected] = useState<Selection>(null);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState<string | null>(null);

  const [pending, setPending] = useState<"rebuild" | "wipe" | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [shortcut, setShortcut] = useState(SUMMON_LABEL);
  // A fresh install has no pages and no way to write one; setup is the only
  // useful thing to show until the /baton command exists.
  const setup = useSetupGate();

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
    api
      .getShortcut()
      .then((a) => setShortcut(prettify(a)))
      .catch(() => {});
  }, [showSettings]);

  // Anything that went wrong before a window existed to say so.
  useEffect(() => {
    api
      .takeNotices()
      .then((queued) => queued.forEach(setToast))
      .catch(() => {});
    const un = api.onNotice(setToast);
    return () => void un.then((f) => f());
  }, []);

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
      setShowSettings(false);
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  const selectedId = selected?.kind === "page" ? selected.page.id : null;

  // Writes into every detected tool, overwriting whatever is there. No detection
  // to go stale, and an overwrite cannot duplicate.
  const installSkills = async () => {
    try {
      const installed = await api.installSkills();
      setToast(
        installed.length
          ? `/baton installed for ${installed.join(", ")}`
          : "No agent tools found to install into",
      );
    } catch (e) {
      setToast(String(e));
    }
  };

  if (setup.needed && setup.status) {
    return (
      <div className="flex h-screen w-screen flex-col bg-white text-stone-900 dark:bg-stone-900 dark:text-stone-100">
        <header
          data-tauri-drag-region
          className="h-11 shrink-0 border-b border-black/10 dark:border-white/10"
        />
        <Setup
          status={setup.status}
          onDone={() => {
            setup.dismiss();
            setup.refresh();
            void reload(query);
          }}
        />
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen flex-col bg-white text-stone-900 dark:bg-stone-900 dark:text-stone-100">
      <header
        data-tauri-drag-region
        className="flex items-center gap-3 border-b border-black/10 px-4 py-2.5 dark:border-white/10"
      >
        <span className="flex items-center gap-2 text-sm font-medium">
          <Logo size={16} className="text-brand" />
          Baton
        </span>

        <div className="relative ml-4">
          <SearchIcon
            size={13}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-stone-400"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search pages"
            className="w-64 rounded-md border border-black/10 bg-black/3 py-1 pl-7 pr-2.5 text-sm outline-none transition-colors duration-150 placeholder:text-stone-400 focus:border-brand/40 focus:bg-transparent dark:border-white/10 dark:bg-white/5 dark:placeholder:text-stone-500"
          />
        </div>

        <div className="ml-auto flex items-center gap-0.5">
          {/* Setup only shows on an empty wiki, so this is the only way back once pages exist. */}
          <IconButton
            label="Install the /baton command into your agent tools"
            onClick={() => void installSkills()}
          >
            <InstallIcon size={15} />
          </IconButton>
          <IconButton
            label="Change the summon shortcut"
            onClick={() => setShowSettings((v) => !v)}
          >
            <SettingsIcon size={15} />
          </IconButton>
          <IconButton
            label="Re-scan ~/Baton for new and edited pages"
            onClick={() => void api.syncWiki().then(() => reload(query))}
          >
            <RefreshIcon size={15} />
          </IconButton>
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <aside className="flex w-60 shrink-0 flex-col overflow-y-auto border-r border-black/10 p-2 dark:border-white/10">
          {pages.length === 0 && (
            <p className="px-2 py-6 text-center text-xs leading-relaxed text-stone-400 dark:text-stone-500">
              {query.trim() ? (
                "No matches."
              ) : (
                <>
                  Nothing filed yet. Run{" "}
                  <code className="font-mono text-brand">/baton</code> at the end of a
                  session.
                </>
              )}
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
              defaultOpen={
                Boolean(query.trim()) || group.some((h) => h.id === selectedId)
              }
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
                  hit={hit}
                />
              ))}
            </ProjectGroup>
          ))}

          <div className="mt-auto border-t border-black/10 pt-3 dark:border-white/10">
            {pending === "rebuild" && (
              <div className="px-1">
                <p className="text-[11px] leading-relaxed text-stone-500 dark:text-stone-400">
                  Rebuild the search index from the files in ~/Baton? Nothing is deleted —
                  the markdown is the source of truth and the index is derived from it.
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
                    className="cursor-pointer rounded bg-stone-900 px-2 py-1 text-[11px] font-medium text-white transition-all duration-150 hover:bg-stone-700 active:scale-[0.98] dark:bg-stone-100 dark:text-stone-900 dark:hover:bg-white"
                  >
                    Rebuild
                  </button>
                  <button
                    onClick={() => setPending(null)}
                    className="cursor-pointer px-1 text-[11px] text-stone-500 transition-all duration-150 hover:underline"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            )}

            {pending === "wipe" && (
              <div className="px-1">
                <p className="text-[11px] leading-relaxed text-stone-500 dark:text-stone-400">
                  Move every project and constraint in ~/Baton to the Trash?
                  {pages.length > 0 &&
                    ` That is ${pages.length} page${pages.length === 1 ? "" : "s"}.`}{" "}
                  Your schema stays, and everything else can be put back from the Trash.
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
              <div className="flex items-center gap-1 px-0.5">
                <FooterAction onClick={() => setPending("rebuild")}>
                  <RefreshIcon size={12} />
                  Rebuild index
                </FooterAction>
                {pages.length > 0 && (
                  <FooterAction danger onClick={() => setPending("wipe")}>
                    <TrashIcon size={12} />
                    Delete all
                  </FooterAction>
                )}
              </div>
            )}
          </div>
        </aside>

        <main className="min-w-0 flex-1">
          {showSettings ? (
            <Settings onNotice={setToast} />
          ) : selected?.kind === "page" ? (
            <PageDetail
              page={selected.page}
              backlinks={selected.backlinks}
              onOpenPage={(id) => void openPage(id)}
              onCopy={async () => {
                try {
                  await api.copyPage(selected.page.id);
                  setToast("Page copied");
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
            <div className="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
              <Logo size={34} className="text-stone-300 dark:text-stone-700" />
              <p className="max-w-xs text-sm leading-relaxed text-stone-400 dark:text-stone-500">
                Pick a page to read it, or press{" "}
                <kbd className="rounded border border-black/10 bg-black/3 px-1 py-px font-mono text-[11px] dark:border-white/10 dark:bg-white/5">
                  {shortcut}
                </kbd>{" "}
                anywhere to copy a whole project.
              </p>
            </div>
          )}
        </main>
      </div>

      {toast && (
        <div className="pointer-events-none fixed bottom-6 left-1/2 -translate-x-1/2 rounded-md bg-stone-900/90 px-3 py-1.5 text-xs text-white shadow-lg dark:bg-stone-100/95 dark:text-stone-900">
          {toast}
        </div>
      )}
    </div>
  );
}

/** An icon-only control. The label is both the tooltip and the accessible name. */
function IconButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip label={label}>
      <button
        onClick={onClick}
        aria-label={label}
        className="cursor-pointer rounded-md p-1.5 text-stone-500 transition-all duration-150 hover:bg-black/5 hover:text-stone-900 active:scale-95 dark:text-stone-400 dark:hover:bg-white/10 dark:hover:text-stone-100"
      >
        {children}
      </button>
    </Tooltip>
  );
}

function FooterAction({
  danger,
  onClick,
  children,
}: {
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex cursor-pointer items-center gap-1.5 rounded px-1.5 py-1 text-[11px] text-stone-400 transition-colors duration-150 dark:text-stone-500 ${
        danger
          ? "hover:bg-red-500/10 hover:text-red-600 dark:hover:text-red-400"
          : "hover:bg-black/5 hover:text-stone-700 dark:hover:bg-white/10 dark:hover:text-stone-200"
      }`}
    >
      {children}
    </button>
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

/** A null slug marks a group that is a heading, not a folder, so cannot be deleted. */
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
        <p className="text-[11px] leading-relaxed text-stone-500 dark:text-stone-400">
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
      {/* A row, not a button: a button cannot nest inside a button. */}
      <div className="group flex items-center rounded pr-1 transition-colors duration-150 hover:bg-black/4 dark:hover:bg-white/5">
        <button
          onClick={() => setOpen((v) => !v)}
          className="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 px-2 py-1 text-left"
        >
          <ChevronIcon
            size={11}
            className={`shrink-0 text-stone-400 transition-transform duration-150 ${open ? "rotate-90" : ""}`}
          />
          <span className="truncate font-mono text-[10px] uppercase tracking-widest text-stone-400 dark:text-stone-500">
            {name}
          </span>
          <span className="tnum ml-auto font-mono text-[10px] text-stone-300 dark:text-stone-600">
            {count}
          </span>
        </button>
        {onDelete && (
          // Hidden until hover: a delete next to a constant control eventually gets hit.
          <button
            onClick={() => setConfirming(true)}
            title={`Delete ${name}`}
            aria-label={`Delete ${name}`}
            className="ml-1 cursor-pointer rounded p-0.5 text-stone-300 opacity-0 transition-all duration-150 hover:text-red-600 group-hover:opacity-100 dark:text-stone-600 dark:hover:text-red-400"
          >
            <TrashIcon size={12} />
          </button>
        )}
      </div>
      {open && (
        <div className="ml-2 border-l border-black/5 pl-1 dark:border-white/5">
          {children}
        </div>
      )}
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
        className="cursor-pointer px-1 text-[11px] text-stone-500 transition-all duration-150 hover:underline"
      >
        Cancel
      </button>
    </div>
  );
}

/** One line per page: the type as a mark, the title, and how stale it is. */
function SidebarRow({
  active,
  onClick,
  hit,
}: {
  active: boolean;
  onClick: () => void;
  hit: PageHit;
}) {
  const faded = hit.status !== "current";
  return (
    <button
      onClick={onClick}
      title={`${TYPE_LABEL[hit.type]} · ${relativeTime(hit.updated)}`}
      className={`group relative mb-px flex w-full cursor-pointer items-center gap-2 rounded-md py-1.5 pl-2.5 pr-2 text-left transition-colors duration-150 before:absolute before:left-0 before:top-1/2 before:h-3.5 before:w-0.5 before:-translate-y-1/2 before:scale-y-0 before:rounded-full before:bg-brand before:transition-transform before:duration-150 ${
        active
          ? "bg-black/6 before:scale-y-100 dark:bg-white/10"
          : "hover:bg-black/4 dark:hover:bg-white/5"
      }`}
    >
      <TypeIcon
        type={hit.type}
        size={13}
        className={`shrink-0 ${active ? "text-brand" : "text-stone-400 dark:text-stone-500"}`}
      />
      <span
        className={`truncate text-[13px] ${faded ? "text-stone-400 dark:text-stone-500" : ""}`}
      >
        {hit.title || readableId(hit.id)}
      </span>
      <span className="tnum ml-auto shrink-0 font-mono text-[10px] text-stone-300 opacity-0 transition-opacity duration-150 group-hover:opacity-100 dark:text-stone-600">
        {relativeTime(hit.updated).replace(" ago", "")}
      </span>
    </button>
  );
}
