/**
 * The entire Rust surface, in one place.
 *
 * Components never call `invoke` directly: keeping it here means the command
 * names and argument shapes are checked in a single file against
 * src-tauri/src/commands.rs, instead of drifting across the component tree.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  BrokenLink,
  IndexReport,
  Page,
  PageHit,
  Primer,
  WikiStatus,
} from "../types";

export const hideLauncher = () => invoke<void>("hide_launcher");

export const openMainWindow = () => invoke<void>("open_main_window");

// ----------------------------------------------------------------- the wiki

/**
 * Bring the index in line with the files under ~/Baton. Cheap to call: the
 * sweep skips every file whose mtime and size match the indexed row.
 */
export const syncWiki = () => invoke<IndexReport>("sync_wiki");

export const listPages = () => invoke<PageHit[]>("list_pages");

export const searchPages = (query: string) =>
  invoke<PageHit[]>("search_pages", { query });

/** Pages that link to this one. */
export const pageBacklinks = (id: string) =>
  invoke<PageHit[]>("page_backlinks", { id });

/** Links that name a page which does not exist. */
export const brokenLinks = () => invoke<BrokenLink[]>("broken_links");

/** The page itself, read from the file rather than from the index. */
export const readPage = (id: string) => invoke<Page>("read_page", { id });

/**
 * The whole project brief, composed from several pages. Cheap enough to rebuild
 * on every summon, so the launcher can show the estimate before copying.
 */
export const buildPrimer = (project?: string) =>
  invoke<Primer>("build_primer", { project: project ?? null });

/** Puts the brief on the clipboard and returns it. */
export const copyPrimer = (project?: string) =>
  invoke<Primer>("copy_primer", { project: project ?? null });

/** Puts the page on the clipboard, frontmatter stripped, and returns it. */
export const copyPage = (id: string) => invoke<string>("copy_page", { id });

/**
 * Fires every time the launcher panel is shown. The window is created once at
 * startup and only ever shown or hidden, so React never remounts and this is
 * the only per-summon signal the webview gets.
 */
export const onLauncherShown = (fn: () => void) => listen("launcher-shown", fn);

/**
 * Fires after the wiki folder changed on disk and was reindexed. Windows
 * refresh from this rather than polling — an agent writing pages mid-session
 * should show up without a restart.
 */
export const onWikiChanged = (fn: () => void) => listen("wiki-changed", fn);

// --- first-run setup ----------------------------------------------------

export const wikiStatus = () => invoke<WikiStatus>("wiki_status");

/** Writes the /baton skill into every detected agent tool. */
export const installSkills = () => invoke<string[]>("install_skills");

export const revealWiki = () => invoke<void>("reveal_wiki");
