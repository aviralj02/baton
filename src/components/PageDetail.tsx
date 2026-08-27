import { useState } from "react";
import type { Page, PageHit, WikiLink } from "../types";
import { relativeTime } from "../lib/time";
import { Dot } from "./Dot";
import { Tooltip } from "./Tooltip";
import {
  CopyIcon,
  FileIcon,
  LinkIcon,
  TrashIcon,
  TypeIcon,
  TYPE_LABEL
} from "./Icon";

/**
 * A wiki page, read from its file. There is no edit mode: pages are written by
 * the agent that did the work, or by hand in an editor, so the only writes this
 * view offers are opening the file in whatever owns `.md`, and deleting it.
 */
export function PageDetail({
  page,
  backlinks,
  onOpenPage,
  onCopy,
  onOpenFile,
  onDelete
}: {
  page: Page;
  backlinks: PageHit[];
  onOpenPage: (id: string) => void;
  onCopy: () => void;
  onOpenFile: () => void;
  onDelete: () => void;
}) {
  const { frontmatter: fm } = page;
  const outgoing = dedupe(page.links);
  const [confirming, setConfirming] = useState(false);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-start justify-between gap-3 border-b border-black/10 px-6 py-4 dark:border-white/10">
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-lg font-medium">
            {page.title || page.id}
          </h1>
          <div className="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-stone-500 dark:text-stone-400">
            <span className="flex items-center gap-1.5 text-stone-600 dark:text-stone-300">
              <TypeIcon type={fm.type} size={13} className="text-brand" />
              {TYPE_LABEL[fm.type]}
            </span>
            {fm.status !== "current" && <Tag>{fm.status}</Tag>}
            {fm.project && (
              <>
                <Dot />
                <span>{fm.project}</span>
              </>
            )}
            <Dot />
            <span>updated {relativeTime(fm.updated)}</span>
            {fm.sources.length > 0 && (
              <>
                <Dot />
                <span>
                  {fm.sources.length} source{fm.sources.length === 1 ? "" : "s"}
                </span>
              </>
            )}
          </div>
          <p className="mt-1.5 truncate font-mono text-[11px] text-stone-400 dark:text-stone-500">
            {page.id}
          </p>
        </div>

        {confirming ? (
          // Replaces the button group so the next click cannot land on Copy.
          <div className="flex shrink-0 items-center gap-2 flex-col">
            <span className="text-xs text-stone-500 dark:text-stone-400">
              Move to Trash?
            </span>
            <div className="flex items-center gap-2">
              <button
                onClick={() => {
                  setConfirming(false);
                  onDelete();
                }}
                className="cursor-pointer rounded-md bg-red-600 px-2.5 py-1 text-xs font-medium text-white transition-all duration-150 hover:bg-red-700 active:scale-[0.98]"
              >
                Delete
              </button>
              <button
                onClick={() => setConfirming(false)}
                className="cursor-pointer px-1 text-xs text-stone-500 transition-all duration-150 hover:underline"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div className="flex shrink-0 items-center gap-1">
            <GhostButton
              label="Move this page to the Trash"
              danger
              onClick={() => setConfirming(true)}
            >
              <TrashIcon size={14} />
            </GhostButton>
            <GhostButton
              label="Open the markdown file in your editor"
              onClick={onOpenFile}
            >
              <FileIcon size={14} />
            </GhostButton>
            <button
              onClick={onCopy}
              className="ml-1 flex cursor-pointer items-center gap-1.5 rounded-md bg-brand px-2.5 py-1.5 text-xs font-medium text-white transition-all duration-300 hover:bg-brand-strong active:scale-[0.98] dark:text-stone-900"
            >
              <CopyIcon size={13} />
              Copy page
            </button>
          </div>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
        {page.preamble && <Prose>{page.preamble}</Prose>}

        {page.sections.map((section, i) => (
          <section key={`${i}-${section.heading}`} className="mb-6">
            <h2 className="mb-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-stone-400 dark:text-stone-500">
              {section.heading}
            </h2>
            <Prose>{section.body}</Prose>
          </section>
        ))}

        <LinkList heading="Links out" empty="This page links nowhere.">
          {outgoing.map((link) => (
            <LinkRow
              key={link.target}
              label={link.alias ?? link.target}
              target={link.target}
              // A target that resolves to no path climbed out of the wiki root.
              broken={link.path === null}
              onOpenPage={onOpenPage}
            />
          ))}
        </LinkList>

        <LinkList heading="Linked from" empty="Nothing links here.">
          {backlinks.map((hit) => (
            <LinkRow
              key={hit.id}
              label={hit.title || hit.id}
              target={hit.id}
              broken={false}
              onOpenPage={onOpenPage}
            />
          ))}
        </LinkList>
      </div>
    </div>
  );
}

/** An icon-only header action. The label is both the tooltip and the accessible name. */
function GhostButton({
  label,
  danger,
  onClick,
  children
}: {
  label: string;
  danger?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip label={label}>
      <button
        onClick={onClick}
        aria-label={label}
        className={`cursor-pointer rounded-md p-1.5 text-stone-400 transition-all duration-150 active:scale-95 dark:text-stone-500 ${
          danger
            ? "hover:bg-red-500/10 hover:text-red-600 dark:hover:text-red-400"
            : "hover:bg-black/5 hover:text-stone-800 dark:hover:bg-white/10 dark:hover:text-stone-100"
        }`}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/** Markdown source, shown as written. The file is the truth, so do not restyle it. */
function Prose({ children }: { children: string }) {
  return (
    <p className="whitespace-pre-wrap text-sm leading-relaxed text-stone-700 dark:text-stone-300">
      {children}
    </p>
  );
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded bg-black/5 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide dark:bg-white/10">
      {children}
    </span>
  );
}

function LinkList({
  heading,
  empty,
  children
}: {
  heading: string;
  empty: string;
  children: React.ReactNode[];
}) {
  return (
    <section className="mb-6">
      <h2 className="mb-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-stone-400 dark:text-stone-500">
        {heading}
      </h2>
      {children.length === 0 ? (
        <p className="text-xs text-stone-400 dark:text-stone-500">{empty}</p>
      ) : (
        <div className="flex flex-col items-start gap-0.5">{children}</div>
      )}
    </section>
  );
}

function LinkRow({
  label,
  target,
  broken,
  onOpenPage
}: {
  label: string;
  target: string;
  broken: boolean;
  onOpenPage: (id: string) => void;
}) {
  if (broken) {
    return (
      <span className="flex items-center gap-1.5 rounded px-1.5 py-0.5 text-sm text-stone-400 line-through dark:text-stone-500">
        <LinkIcon size={12} />
        {label}
      </span>
    );
  }
  return (
    <button
      onClick={() => onOpenPage(target)}
      className="group flex cursor-pointer items-center gap-1.5 rounded px-1.5 py-0.5 text-left text-sm text-stone-700 transition-all duration-150 hover:bg-black/5 active:scale-[0.98] dark:text-stone-300 dark:hover:bg-white/10"
    >
      <LinkIcon
        size={12}
        className="text-stone-300 transition-colors duration-150 group-hover:text-brand dark:text-stone-600"
      />
      {label}
    </button>
  );
}

/** Prose repeats a link. The list needs it once. */
function dedupe(links: WikiLink[]): WikiLink[] {
  const seen = new Set<string>();
  return links.filter((link) => {
    if (seen.has(link.target)) return false;
    seen.add(link.target);
    return true;
  });
}
