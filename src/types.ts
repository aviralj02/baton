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
