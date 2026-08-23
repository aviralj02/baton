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

export interface Primer {
  project: string;
  text: string;
  /** Estimated, not measured. Shown so a budget can be judged before pasting. */
  tokens: number;
  pagesIncluded: number;
  pagesDropped: number;
}

/** First-run state — mirrors `onboarding::WikiStatus`. */
export interface WikiStatus {
  root: string;
  rootExists: boolean;
  hasSchema: boolean;
  pageCount: number;
  skills: SkillHost[];
}

export interface SkillHost {
  name: string;
  /** The tool's config directory exists on this machine. */
  detected: boolean;
  installed: boolean;
  /** Installed, but differs from the version this build ships. */
  outdated: boolean;
}

/** A project row in the launcher — mirrors `db::ProjectHit`. */
export interface ProjectHit {
  slug: string;
  title: string;
  pageCount: number;
  updated: string;
}
