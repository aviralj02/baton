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
  IndexReport,
  Page,
  PageHit,
  Primer,
  ProjectHit,
  WikiStatus,
} from "../types";

export const hideLauncher = () => invoke<void>("hide_launcher");

export const openMainWindow = () => invoke<void>("open_main_window");

// ----------------------------------------------------------------- the wiki

/** Clears the index and rebuilds it from the files. Never touches the files. */
export const rebuildIndex = () => invoke<IndexReport>("rebuild_index");

/**
 * Bring the index in line with the files under ~/Baton. Cheap to call: the
 * sweep skips every file whose mtime and size match the indexed row.
 */
export const syncWiki = () => invoke<IndexReport>("sync_wiki");

// Deleting. These are the only calls that touch the markdown, and every one of
// them moves files to the OS trash rather than unlinking them, so a mis-click
// is recovered from the Finder or the Recycle Bin. Each returns the index as it
// stands afterwards.

/** Moves one page's file to the trash. */
export const deletePage = (id: string) => invoke<IndexReport>("delete_page", { id });

/** Moves a whole project — every page under it — to the trash. */
export const deleteProject = (slug: string) =>
  invoke<IndexReport>("delete_project", { slug });

/** Moves every project and constraint to the trash. The schema survives. */
export const deleteEverything = () => invoke<IndexReport>("delete_everything");

/**
 * Projects for the launcher — one row per project, never per page. A project's
 * pages are an organisational detail of the wiki folder; what a user summons
 * Baton for is "give me everything about X".
 */
export const listProjects = () => invoke<ProjectHit[]>("list_projects");

/** Matches a project name or a page title. Deliberately not body text. */
export const searchProjects = (query: string) =>
  invoke<ProjectHit[]>("search_projects", { query });

export const listPages = () => invoke<PageHit[]>("list_pages");

export const searchPages = (query: string) =>
  invoke<PageHit[]>("search_pages", { query });

/** Pages that link to this one. */
export const pageBacklinks = (id: string) => invoke<PageHit[]>("page_backlinks", { id });

/** The page itself, read from the file rather than from the index. */
export const readPage = (id: string) => invoke<Page>("read_page", { id });

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

// --- settings -----------------------------------------------------------

/** The summon shortcut as an accelerator string, e.g. "CmdOrCtrl+Shift+Space". */
export const getShortcut = () => invoke<string>("get_shortcut");

/** Registers and persists a new summon shortcut. Rejects one already taken. */
export const setShortcut = (accelerator: string) =>
  invoke<void>("set_shortcut", { accelerator });

/** Problems recorded before a window existed to show them. Drains the queue. */
export const takeNotices = () => invoke<string[]>("take_notices");

/** Problems raised while a window is open. */
export const onNotice = (fn: (message: string) => void) =>
  listen<string>("baton://notice", (e) => fn(e.payload));

// --- first-run setup ----------------------------------------------------

export const wikiStatus = () => invoke<WikiStatus>("wiki_status");

/** Writes the /baton skill into every detected agent tool. */
export const installSkills = () => invoke<string[]>("install_skills");

export const revealWiki = () => invoke<void>("reveal_wiki");
