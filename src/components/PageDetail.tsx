import type { Page, PageHit, WikiLink } from "../types";
import { relativeTime } from "../Launcher";
import { Dot } from "./Dot";

/**
 * A wiki page, read from its file. There is no edit mode: pages are written by
 * the agent that did the work, or by hand in an editor, so the only write this
 * view offers is opening the file in whatever owns `.md`.
 */
export function PageDetail({
  page,
  backlinks,
  onOpenPage,
  onCopy,
  onOpenFile,
}: {
  page: Page;
  backlinks: PageHit[];
  onOpenPage: (id: string) => void;
  onCopy: () => void;
  onOpenFile: () => void;
}) {
  const { frontmatter: fm } = page;
  const outgoing = dedupe(page.links);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-start gap-3 border-b border-black/10 px-6 py-4 dark:border-white/10">
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-lg font-medium">{page.title || page.id}</h1>
          <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-neutral-500 dark:text-neutral-400">
            <Tag>{fm.type}</Tag>
            {fm.status !== "current" && <Tag>{fm.status}</Tag>}
            {fm.project && <span>{fm.project}</span>}
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
          <p className="mt-1 truncate font-mono text-[11px] text-neutral-400">{page.id}</p>
        </div>

        <button
          onClick={onOpenFile}
          className="shrink-0 cursor-pointer rounded-md px-2.5 py-1 text-xs transition-all duration-150 hover:bg-black/5 active:scale-[0.98] dark:text-neutral-300 dark:hover:bg-white/10"
        >
          Open file
        </button>
        <button
          onClick={onCopy}
          className="shrink-0 cursor-pointer rounded-md bg-neutral-900 px-2.5 py-1 text-xs font-medium text-white transition-all duration-150 hover:bg-neutral-700 active:scale-[0.98] dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
        >
          Copy
        </button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
        {page.preamble && <Prose>{page.preamble}</Prose>}

        {page.sections.map((section, i) => (
          <section key={`${i}-${section.heading}`} className="mb-6">
            <h2 className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-neutral-400">
              {section.heading}
            </h2>
            <Prose>{section.body}</Prose>
          </section>
        ))}

        <LinkList heading="Links out" empty="This page links nowhere." onOpenPage={onOpenPage}>
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

        <LinkList heading="Linked from" empty="Nothing links here." onOpenPage={onOpenPage}>
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

/** Markdown source, shown as written. The file is the truth, so do not restyle it. */
function Prose({ children }: { children: string }) {
  return (
    <p className="whitespace-pre-wrap text-sm leading-relaxed text-neutral-700 dark:text-neutral-300">
      {children}
    </p>
  );
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded bg-black/5 px-1.5 py-0.5 text-[10px] uppercase tracking-wide dark:bg-white/10">
      {children}
    </span>
  );
}


function LinkList({
  heading,
  empty,
  children,
}: {
  heading: string;
  empty: string;
  onOpenPage: (id: string) => void;
  children: React.ReactNode[];
}) {
  return (
    <section className="mb-6">
      <h2 className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-neutral-400">
        {heading}
      </h2>
      {children.length === 0 ? (
        <p className="text-xs text-neutral-400">{empty}</p>
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
  onOpenPage,
}: {
  label: string;
  target: string;
  broken: boolean;
  onOpenPage: (id: string) => void;
}) {
  if (broken) {
    return (
      <span className="rounded px-1.5 py-0.5 text-sm text-neutral-400 line-through">{label}</span>
    );
  }
  return (
    <button
      onClick={() => onOpenPage(target)}
      className="cursor-pointer rounded px-1.5 py-0.5 text-left text-sm text-neutral-700 transition-all duration-150 hover:bg-black/5 active:scale-[0.98] dark:text-neutral-300 dark:hover:bg-white/10"
    >
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
