/**
 * Mirrors the Rust `Context` struct (src-tauri/src/context.rs). The Rust side
 * flattens `body`, so this stays one flat object across the IPC boundary.
*/
export interface Context {
  id: string;
  name: string;
  description: string | null;
  goal: string | null;
  currentState: string | null;
  decisions: string[];
  tried: string[];
  relevantFiles: string[];
  constraints: string[];
  openIssues: string[];
  nextSteps: string[];
  createdAt: string;
  updatedAt: string;
}

export interface ContextSummary {
  id: string;
  name: string;
  updatedAt: string;
}

/** The editable half of a context, as `save_context` expects it. */
export type ContextBody = Omit<Context, "id" | "name" | "createdAt" | "updatedAt">;

/**
 * The wiki. Pages are markdown files under ~/Baton and are the source of truth.
 * Mirrors src-tauri/src/wiki.rs for the page itself, and the index half of
 * src-tauri/src/db.rs for everything derived from it.
 */
export type PageType =
  | "project"
  | "decision"
  | "open"
  | "attempt"
  | "component"
  | "gotcha";

export type PageStatus = "current" | "superseded" | "abandoned" | "stale";

/** One row of the index: enough to render a result, never the page itself. */
export interface PageHit {
  id: string;
  path: string;
  title: string;
  type: PageType;
  project: string | null;
  status: PageStatus;
  updated: string;
  /** Matched body text. Empty when the hit did not come from a query. */
  snippet: string;
}

export interface PageFrontmatter {
  type: PageType;
  project: string | null;
  status: PageStatus;
  updated: string;
  sources: string[];
}

export interface PageSection {
  heading: string;
  body: string;
}

export interface WikiLink {
  target: string;
  alias: string | null;
  /** Null when the target climbs out of the wiki root. */
  path: string | null;
}

export interface Page {
  id: string;
  path: string;
  frontmatter: PageFrontmatter;
  title: string;
  preamble: string;
  sections: PageSection[];
  links: WikiLink[];
  body: string;
}

export interface IndexReport {
  indexed: number;
  skipped: number;
  removed: number;
  /** One entry per page that could not be read. It keeps its existing rows. */
  errors: string[];
}

export interface BrokenLink {
  src: string;
  dst: string;
}

export const EMPTY_BODY: ContextBody = {
  description: null,
  goal: null,
  currentState: null,
  decisions: [],
  tried: [],
  relevantFiles: [],
  constraints: [],
  openIssues: [],
  nextSteps: [],
};
