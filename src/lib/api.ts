/**
 * The entire Rust surface, in one place.
 *
 * Components never call `invoke` directly: keeping it here means the command
 * names and argument shapes are checked in a single file against
 * src-tauri/src/commands.rs, instead of drifting across the component tree.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  BrokenLink,
  Context,
  ContextBody,
  ContextSummary,
  IndexReport,
  Page,
  PageHit,
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

/** Extract a structured context from a pasted conversation, and save it. */
export const createFromConversation = (name: string, conversation: string) =>
  invoke<Context>("create_context_from_conversation", { name, conversation });

/** Merge a newer conversation into an existing context. */
export const updateFromConversation = (id: string, conversation: string) =>
  invoke<Context>("update_context_from_conversation", { id, conversation });

/** Write a continuation prompt for the next model, and copy it. */
export const generateHandoff = (id: string) =>
  invoke<string>("generate_handoff", { id });
