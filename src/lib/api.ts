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
  Context,
  ContextBody,
  ContextSummary,
  IndexReport,
  Page,
  PageHit,
  Primer,
} from "../types";

export const listContexts = () => invoke<ContextSummary[]>("list_contexts");

export const searchContexts = (query: string) =>
  invoke<ContextSummary[]>("search_contexts", { query });

export const getContext = (id: string) => invoke<Context>("get_context", { id });

export const saveContext = (args: {
  id?: string | null;
  name: string;
  body?: ContextBody;
}) => invoke<Context>("save_context", args);

export const deleteContext = (id: string) => invoke<void>("delete_context", { id });

export const deleteAllData = () => invoke<void>("delete_all_data");

/** Renders to markdown AND writes it to the clipboard, in one hop. */
export const copyContext = (id: string) => invoke<string>("copy_context", { id });

/** Renders to markdown without touching the clipboard. */
export const renderContext = (id: string) => invoke<string>("render_context", { id });

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

/** Extract a structured context from a pasted conversation, and save it. */
export const createFromConversation = (name: string, conversation: string) =>
  invoke<Context>("create_context_from_conversation", { name, conversation });

/** Merge a newer conversation into an existing context. */
export const updateFromConversation = (id: string, conversation: string) =>
  invoke<Context>("update_context_from_conversation", { id, conversation });

/** Write a continuation prompt for the next model, and copy it. */
export const generateHandoff = (id: string) =>
  invoke<string>("generate_handoff", { id });
