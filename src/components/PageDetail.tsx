import type { Page, PageHit, WikiLink } from "../types";
import { relativeTime } from "../lib/time";
import { Dot } from "./Dot";
import { Button, IconButton } from "./Button";
import { Label, SectionHeading } from "./Label";
import { CopyIcon, FileIcon, LinkIcon, TrashIcon, TypeIcon, TYPE_LABEL } from "./Icon";

/**
 * A wiki page, read from its file. There is no edit mode: pages are written by
 * the agent that did the work, or by hand in an editor, so the only writes this
 * view offers are opening the file in whatever owns `.md`, and deleting it.
 *
 * It is laid out as something to read rather than as a record to inspect: one
 * column at a measure that stays readable on a wide window, a serif title, and
 * headings that are found by their weight in the page rather than by size.
 */
export function PageDetail({
  page,
  backlinks,
  onOpenPage,
  onCopy,
  onOpenFile,
  onDelete,
}: {
  page: Page;
  backlinks: PageHit[];
  onOpenPage: (id: string) => void;
  onCopy: () => void;
  onOpenFile: () => void;
  /** Opens the window's confirm dialog. Nothing is deleted by this click. */
  onDelete: () => void;
}) {
  const { frontmatter: fm } = page;
  const outgoing = dedupe(page.links);

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-start justify-between gap-6 border-b border-line px-8 py-5">
        <div className="min-w-0 flex-1">
          <h1 className="truncate font-serif text-display tracking-tight text-ink">
            {page.title || page.id}
          </h1>
          <div className="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-ui text-muted">
            <span className="flex items-center gap-1.5 text-body">
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
            <Dot />
            {/* The id is what a [[link]] would name, so it stays visible; long
                ones truncate and the full value is on hover. */}
            <span
              title={page.id}
              className="min-w-0 truncate font-mono text-meta text-faint"
            >
              {page.id}
            </span>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          <IconButton
            label="Move this page to the Trash"
            danger
            haspopup
            onClick={onDelete}
          >
            <TrashIcon size={14} />
          </IconButton>
          <IconButton label="Open the markdown file in your editor" onClick={onOpenFile}>
            <FileIcon size={14} />
          </IconButton>
          <span className="ml-1">
            <Button variant="primary" onClick={onCopy}>
              <CopyIcon size={13} />
              Copy page
            </Button>
          </span>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-8">
        {/* A measure, not the window width: prose set edge to edge is unreadable. */}
        <article className="mx-auto max-w-2xl">
          {page.preamble && <Prose>{page.preamble}</Prose>}

          {page.sections.map((section, i) => (
            <section key={`${i}-${section.heading}`} className="mt-10">
              <SectionHeading>{section.heading}</SectionHeading>
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
        </article>
      </div>
    </div>
  );
}

/** Markdown source, shown as written. The file is the truth, so do not restyle it. */
function Prose({ children }: { children: string }) {
  return <p className="whitespace-pre-wrap text-read text-body">{children}</p>;
}

function Tag({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded bg-panel px-1.5 py-0.5 font-mono text-micro uppercase tracking-wide text-body">
      {children}
    </span>
  );
}

/**
 * Links are the shape of the wiki, so they get rows rather than chips: full
 * width, one per line, a rule between them. A row is also a target you can hit
 * without aiming.
 */
function LinkList({
  heading,
  empty,
  children,
}: {
  heading: string;
  empty: string;
  children: React.ReactNode[];
}) {
  return (
    <section className="mt-10">
      <Label>{heading}</Label>
      {children.length === 0 ? (
        <p className="mt-2 text-ui text-muted">{empty}</p>
      ) : (
        <div className="-mx-3 mt-1">{children}</div>
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
      <span className="flex items-center gap-2 border-b border-line-soft px-3 py-2.5 text-read text-muted line-through">
        <LinkIcon size={13} className="shrink-0" />
        {label}
      </span>
    );
  }
  return (
    <button
      onClick={() => onOpenPage(target)}
      className="group flex w-full cursor-pointer items-center gap-2 border-b border-line-soft px-3 py-2.5 text-left text-read text-body transition-all duration-150 hover:bg-hover hover:text-ink active:scale-[0.98]"
    >
      <LinkIcon
        size={13}
        className="shrink-0 text-faint transition-colors duration-150 group-hover:text-brand"
      />
      <span className="truncate">{label}</span>
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
