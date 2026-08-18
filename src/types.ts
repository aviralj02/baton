/**
 * Mirrors the Rust `Context` struct. Stored as JSON in the `contexts.content`
 * column — markdown is a rendered representation, never the source of truth.
 * See PRD.md section "Data model".
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
