/**
 * The entire Rust surface, in one place.
 *
 * Components never call `invoke` directly: keeping it here means the command
 * names and argument shapes are checked in a single file against
 * src-tauri/src/commands.rs, instead of drifting across the component tree.
 */
import { invoke } from "@tauri-apps/api/core";
import type { Context, ContextBody, ContextSummary } from "../types";

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
