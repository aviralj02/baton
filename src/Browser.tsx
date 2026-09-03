import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import * as api from "./lib/api";
import { relativeTime } from "./lib/time";
import { PageDetail } from "./components/PageDetail";
import { Setup, useSetupGate } from "./components/Setup";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { Settings } from "./components/Settings";
import { Logo } from "./components/Logo";
import { IconButton } from "./components/Button";
import { Shortcut } from "./components/Shortcut";
import { DEFAULT_SHORTCUT } from "./lib/platform";
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

/**
 * A pending confirmation. Every consequential action in this window describes
 * itself as one of these and hands over the work, so there is one dialog on the
 * page rather than a confirmation built into each control that needs one.
 *
 * `run` lets its errors escape: the dialog closes on success and stays open on
 * failure, which is only possible if failure reaches the caller.
 */
type Confirm = {
  title: string;
  body: string;
  confirmLabel: string;
  variant: "primary" | "danger";
  run: () => Promise<void>;
};

export default function Browser() {
  const [pages, setPages] = useState<PageHit[]>([]);
  const [selected, setSelected] = useState<Selection>(null);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState<string | null>(null);

  const [confirm, setConfirm] = useState<Confirm | null>(null);
  const [running, setRunning] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [shortcut, setShortcut] = useState(DEFAULT_SHORTCUT);
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
      .then(setShortcut)
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

  const runConfirmed = async () => {
    if (!confirm) return;
    setRunning(true);
    try {
      await confirm.run();
      setConfirm(null);
    } catch (e) {
      setToast(String(e));
    } finally {
      setRunning(false);
    }
  };

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
      <div className="flex h-screen w-screen flex-col bg-surface text-ink">
        <header data-tauri-drag-region className="h-11 shrink-0 border-b border-line" />
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
    <div className="flex h-screen w-screen flex-col bg-surface text-ink">
      <header
        data-tauri-drag-region
        className="flex h-12 shrink-0 items-center gap-3 border-b border-line px-4"
      >
        <span className="flex items-center gap-2">
          <Logo size={16} className="text-brand" />
          <span className="font-serif text-title tracking-tight">Baton</span>
        </span>

        <div className="relative ml-4">
          <SearchIcon
            size={13}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search pages"
            className="h-8 w-64 rounded-md border border-line bg-panel pl-7 pr-2.5 text-ui text-ink outline-none transition-all duration-200 placeholder:text-ui placeholder:text-muted focus:border-brand/45 focus:bg-surface"
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
        <aside className="flex w-64 shrink-0 flex-col overflow-y-auto border-r border-line p-3">
          {pages.length === 0 && (
            <p className="px-2 py-6 text-center text-ui leading-relaxed text-muted">
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
                  ? () =>
                      setConfirm({
                        title: `Move ${name} to the Trash?`,
                        body: `${name} and its ${group.length} ${
                          group.length === 1 ? "page" : "pages"
                        } go to the Trash. You can put them back from there.`,
                        confirmLabel: "Move to Trash",
                        variant: "danger",
                        run: async () => {
                          // The open page may have been inside the project.
                          if (group.some((h) => h.id === selectedId)) setSelected(null);
                          await api.deleteProject(slug);
                          await reload(query);
                          setToast(`${name} moved to Trash`);
                        },
                      })
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

          <div className="mt-auto flex items-center gap-1 border-t border-line pt-3">
            <FooterAction
              onClick={() =>
                setConfirm({
                  title: "Rebuild the search index?",
                  body: "Nothing is deleted. The markdown in ~/Baton is the source of truth, and the index is read back from it.",
                  confirmLabel: "Rebuild",
                  variant: "primary",
                  run: async () => {
                    setSelected(null);
                    const report = await api.rebuildIndex();
                    await reload(query);
                    setToast(`Index rebuilt from ${report.indexed} pages`);
                  },
                })
              }
            >
              <RefreshIcon size={12} />
              Rebuild index
            </FooterAction>
            {pages.length > 0 && (
              <FooterAction
                danger
                onClick={() =>
                  setConfirm({
                    title: "Move everything to the Trash?",
                    body: `Every project and constraint in ~/Baton, ${pages.length} ${
                      pages.length === 1 ? "page" : "pages"
                    } in all, goes to the Trash. Your schema stays, and anything can be put back.`,
                    confirmLabel: "Move all to Trash",
                    variant: "danger",
                    run: async () => {
                      setSelected(null);
                      await api.deleteEverything();
                      await reload(query);
                      setToast("Everything moved to Trash");
                    },
                  })
                }
              >
                <TrashIcon size={12} />
                Delete all
              </FooterAction>
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
              onDelete={() => {
                const page = selected.page;
                const title = page.title || readableId(page.id);
                setConfirm({
                  title: `Move ${title} to the Trash?`,
                  body: "The markdown file goes to the Trash, and the page leaves the index. You can put it back from there.",
                  confirmLabel: "Move to Trash",
                  variant: "danger",
                  run: async () => {
                    setSelected(null);
                    await api.deletePage(page.id);
                    await reload(query);
                    setToast(`${title} moved to Trash`);
                  },
                });
              }}
            />
          ) : (
            // The shortcut sits on its own line rather than inside the sentence.
            // A keycap set into running text stretches that one line and leaves
            // the next one tight, which is what made this read as cramped.
            <div className="flex h-full flex-col items-center justify-center px-8 text-center">
              <Logo size={32} className="text-muted" />
              <p className="mt-6 font-serif text-title tracking-tight text-body">
                Pick a page to read it.
              </p>
              <div className="mt-7 flex items-center gap-2.5">
                <Shortcut accelerator={shortcut} />
                <span className="text-ui text-muted">
                  copies a whole project, from any app
                </span>
              </div>
            </div>
          )}
        </main>
      </div>

      <ConfirmDialog
        open={confirm !== null}
        title={confirm?.title ?? ""}
        body={confirm?.body ?? ""}
        confirmLabel={confirm?.confirmLabel ?? ""}
        variant={confirm?.variant}
        pending={running}
        onConfirm={() => void runConfirmed()}
        onCancel={() => setConfirm(null)}
      />

      {toast && (
        <div className="pointer-events-none fixed bottom-6 left-1/2 -translate-x-1/2 rounded-full bg-ink px-3.5 py-1.5 text-ui text-surface shadow-lg">
          {toast}
        </div>
      )}
    </div>
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
      aria-haspopup="dialog"
      className={`flex cursor-pointer items-center gap-1.5 rounded px-1.5 py-1 text-meta text-muted transition-all duration-150 active:scale-[0.98] ${
        danger
          ? "hover:bg-danger-soft hover:text-danger"
          : "hover:bg-hover hover:text-body"
      }`}
    >
      {children}
    </button>
  );
}

/**
 * A readable stand-in for a page with no title: its last path segment with the
 * hyphens opened out. Pages should carry a `#` heading, and lint flags the ones
 * that do not, but a path in the sidebar is worse than an imperfect name.
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
  onDelete?: () => void;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);

  // A search result that lands in a closed group would be invisible.
  useEffect(() => {
    if (defaultOpen) setOpen(true);
  }, [defaultOpen]);

  return (
    <section className="mb-1">
      {/* A row, not a button: a button cannot nest inside a button. */}
      <div className="group flex items-center rounded pr-1 transition-colors duration-150 hover:bg-hover">
        <button
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
          className="flex h-7 min-w-0 flex-1 cursor-pointer items-center gap-1.5 px-2 text-left transition-all duration-150 active:scale-[0.98]"
        >
          <ChevronIcon
            size={11}
            className={`shrink-0 text-faint transition-transform duration-150 ${open ? "rotate-90" : ""}`}
          />
          <span className="truncate font-mono text-micro uppercase tracking-[0.14em] text-muted">
            {name}
          </span>
          <span className="tnum ml-auto font-mono text-micro text-faint">{count}</span>
        </button>
        {onDelete && (
          // Hidden until hover: a delete next to a constant control eventually gets hit.
          // A native title rather than Tooltip, because this list scrolls and an
          // overflow-y-auto parent clips both axes.
          <button
            onClick={onDelete}
            title={`Delete ${name}`}
            aria-label={`Delete ${name}`}
            aria-haspopup="dialog"
            className="ml-1 cursor-pointer rounded p-0.5 text-faint opacity-0 transition-all duration-150 hover:text-danger active:scale-95 group-hover:opacity-100"
          >
            <TrashIcon size={12} />
          </button>
        )}
      </div>
      {open && <div className="mt-1 ml-2 border-l border-line-soft pl-1">{children}</div>}
    </section>
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
      title={`${TYPE_LABEL[hit.type]}, ${relativeTime(hit.updated)}`}
      className={`group relative mb-px flex h-8 w-full cursor-pointer items-center gap-2 rounded-md pl-2.5 pr-2 text-left transition-all duration-150 active:scale-[0.98] before:absolute before:left-0 before:top-1/2 before:h-3.5 before:w-0.5 before:-translate-y-1/2 before:scale-y-0 before:rounded-full before:bg-brand before:transition-transform before:duration-150 ${
        active ? "bg-active before:scale-y-100" : "hover:bg-hover"
      }`}
    >
      <TypeIcon
        type={hit.type}
        size={13}
        className={`shrink-0 ${active ? "text-brand" : "text-muted"}`}
      />
      <span className={`truncate text-ui ${faded ? "text-muted" : "text-body"}`}>
        {hit.title || readableId(hit.id)}
      </span>
      <span className="tnum ml-auto shrink-0 font-mono text-micro text-faint opacity-0 transition-opacity duration-150 group-hover:opacity-100">
        {relativeTime(hit.updated).replace(" ago", "")}
      </span>
    </button>
  );
}
