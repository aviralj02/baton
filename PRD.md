# Baton — PRD & Build Plan (v0)

**Name:** Baton
**Platforms:** macOS + Windows (Tauri v2)
**Target user:** Developers working across Claude, ChatGPT, Codex, Cursor, Gemini
**Product type:** Local-first desktop utility / Raycast-style launcher

> This document is the single source of truth for v0. It merges the original
> `local-context-manager-v0-prd.docx` with the architecture decisions made
> during scaffolding. Where the two conflict, **this document wins** — the
> conflicts are called out explicitly under "Decisions made during scaffolding".

---

## 1. Product thesis

Developers increasingly work across multiple AI tools, but context is trapped
inside individual conversations. When switching models or starting a new chat,
developers manually summarize previous conversations, copy/paste important
messages, explain decisions again, remember what was already tried, reconstruct
project state, and feed the new model files/context again.

The product provides a persistent local context layer that lets developers
capture, organize, update, and quickly paste context into any AI tool.

**Positioning:** *Your context, independent of the AI you're using.*

Do not position v0 as another generic AI-memory product. The mental model is
**a local, persistent context layer for AI-assisted development**. The clipboard
is the interaction; the persistent context layer is the product.

---

## 2. v0 goal

Prove one workflow:

> I'm working on something with an AI → I need to start a new chat → I can
> create a useful handoff in seconds → paste it into another AI → continue
> immediately.

**Primary success metric:** a developer goes from existing conversation →
persistent context → new AI conversation in **under 30 seconds**, excluding LLM
generation time.

**Secondary metric worth tracking (locally, opt-in):** how often a user *edits*
a generated context before copying it. Speed is fixable; extraction quality is
the real risk. If users routinely rewrite the output, the prompts are wrong and
no amount of latency work will save the product.

---

## 3. What we are NOT building

AI chat client · browser extension · Claude/ChatGPT/Codex integrations · vector
database · cloud sync · accounts · team collaboration · autonomous agents ·
automatic conversation scraping · IDE integration · automatic Git analysis ·
complex knowledge graphs · mobile app.

v0 is intentionally: **local context + beautiful launcher + LLM-powered handoff
+ clipboard.**

---

## 4. Core user flows

### Flow A — Create a context

```
⌘⇧Space (Ctrl⇧Space on Windows)
┌─────────────────────────────────────┐
│ 🔍  Search or create context...     │
├─────────────────────────────────────┤
│  Create context                     │
│  Create handoff from conversation   │
│                                     │
│  Recent                             │
│  ─────────────────────────────      │
│  Auth migration            3h ago   │
│  Stripe integration      yesterday  │
│  Dashboard redesign        Aug 10   │
└─────────────────────────────────────┘
```

User selects **Create context**, enters a name (e.g. "Auth migration"), and
pastes an existing AI conversation. The app sends it to Claude and generates a
structured context.

### Flow B — Handoff (the v0 magic moment)

```
⌘⇧Space → Create handoff → Paste conversation → Generate → Copy
        → Open another AI → ⌘V
```

Generated handoffs must tell the next model that this is **continuation
context**, preserve current state, avoid repeating completed work, and continue
from the next step.

### Flow C — Update an existing context

User selects an existing context and pastes a newer conversation. The LLM
receives the current context **plus** the new conversation and returns an
updated canonical context.

Update rules:
- Preserve valid existing information.
- Update outdated information.
- Remove contradicted information.
- Retain important decisions.
- Prefer newer explicit information when resolving contradictions.
- Do not invent facts.
- Distinguish known facts from assumptions where relevant.

### Flow D — Copy context (the killer action)

```
Auth migration
────────────────────────────
Goal
Current state
Decisions
Open issues
Next steps

⌘↵  Copy context
⌘E  Edit
⌘U  Update
⌘D  Delete
```

`⌘↵` / `Ctrl↵` renders the context to markdown and writes it to the system
clipboard. The user pastes into any AI tool.

---

## 5. Context format (rendered markdown)

The product creates a developer-oriented context document, not a generic
conversation summary.

```markdown
# Auth Migration

## Goal
Replace the existing authentication flow with OAuth while keeping the
current session architecture.

## Current State
- Google OAuth is working locally.
- GitHub OAuth has been partially implemented.
- Production callback is currently failing.

## Decisions
- Don't use NextAuth.
- Keep the existing session model.
- OAuth providers should remain replaceable.

## Things Tried
- Attempted callback URL configuration.
- Tested locally.
- Previous approach failed because of production redirect configuration.

## Relevant Files
- src/auth/*
- src/middleware.ts
- src/api/session.ts

## Open Issues
- GitHub callback fails in production.
- Need to verify production redirect URI.

## Next Step
Debug the production callback configuration.
```

---

## 6. Data model

**This resolves a contradiction in the original PRD.** §6 of the docx described
a rich structured `Context` object; §15 described a flat
`contexts(id, name, content, …)` table. Both are kept, as follows:

- The **table stays flat** (§15).
- The `content` column holds the **structured object as JSON** (§6) — *not*
  markdown.
- **Markdown is generated on demand**, only at the clipboard boundary.

Storing markdown as the source of truth is the trap: every "update context"
call would then require re-parsing prose back into fields.

### SQLite schema

```sql
CREATE TABLE contexts (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  content    TEXT NOT NULL,   -- JSON, see Context struct below
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE sources (
  id         TEXT PRIMARY KEY,
  context_id TEXT NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
  type       TEXT NOT NULL,   -- 'conversation' | 'manual'
  content    TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE contexts_fts USING fts5(
  name, content, content='contexts', content_rowid='rowid'
);
-- plus INSERT/UPDATE/DELETE triggers to keep the index in sync
```

Keep the schema intentionally small. **Do not** prematurely normalize
decisions, files, issues, memories, entities, relationships, or embeddings.

### Context object

Rust struct in `src-tauri/src/`, mirrored by `src/types.ts`:

```
Context
├── id
├── name
├── description
├── goal
├── currentState
├── decisions[]
├── tried[]
├── relevantFiles[]
├── constraints[]
├── openIssues[]
├── nextSteps[]
├── createdAt
└── updatedAt
```

`sourceConversations` and `rawContext` from the original PRD live in the
`sources` table, not on the struct.

---

## 7. Search

Global shortcut opens search. **Plain text search only — embeddings are
explicitly deferred.**

Use **SQLite FTS5** rather than `LIKE '%q%'`. It is the same amount of code,
gives ranked and prefix matching, and is still nowhere near a vector database.

---

## 8. Architecture

```
                  Tauri v2 app
                       │
        ┌──────────────┴──────────────┐
        │                             │
   Webview (React)              Rust core
   - cmdk launcher              - SQLite (rusqlite)
   - context viewer             - OS keystore (keyring)
   - pure UI                    - Anthropic client (reqwest)
                                - global shortcut, tray, window
```

### The rule that governs this codebase

**Rust owns everything sensitive. The webview is pure UI.**

- The API key lives in the OS keystore, is read only in Rust, and is **never
  sent to the frontend**.
- The Anthropic HTTP call happens in Rust.
- SQLite is accessed only from Rust.
- The frontend calls `invoke("create_context", { conversation })` and receives a
  finished `Context` object.

This is not tidiness. If the key or a raw conversation ever transits the
webview, any future JS dependency can reach it, and the local-first privacy
promise the product is positioned on is quietly broken.

### Stack

| Layer | Choice |
|---|---|
| Shell | Tauri v2 |
| Frontend | React 19 + TypeScript + Vite 7 |
| Launcher UI | `cmdk` |
| Styling | Tailwind CSS v4 (`@tailwindcss/vite`) |
| Database | `rusqlite` (feature `bundled`, includes FTS5) |
| Secrets | `keyring` — macOS Keychain / Windows Credential Manager |
| HTTP | `reqwest` (feature `json`) |
| Hotkey | `tauri-plugin-global-shortcut` |
| Clipboard | `tauri-plugin-clipboard-manager` |
| Single instance | `tauri-plugin-single-instance` |
| Launch at login | `tauri-plugin-autostart` |

---

## 9. Local-first architecture & privacy

Everything is stored locally by default. The only content that leaves the
machine is content explicitly sent to the selected LLM provider.

- No account in v0.
- No cloud database.
- No telemetry in v0.
- No automatic cloud sync.
- Contexts stored locally.
- **Raw conversations are never logged.**
- Only explicit LLM actions send content to the configured provider.

### Security requirements

- Store the API key in the OS keystore (`keyring` crate).
- Never log raw conversations — this includes `dbg!`, `println!`, and error
  messages that embed request bodies.
- Provide clear delete-context functionality.
- Provide a delete-all-data action.

---

## 10. LLM operations

No official Anthropic Rust SDK exists, so this is `reqwest` against
`POST https://api.anthropic.com/v1/messages`.

**Headers**
```
x-api-key: <from OS keystore>
anthropic-version: 2023-06-01
content-type: application/json
```

**Request**
```json
{
  "model": "claude-opus-5",
  "max_tokens": 16000,
  "system": "<one of the three prompts below>",
  "output_config": { "effort": "medium" },
  "messages": [{ "role": "user", "content": "..." }]
}
```

Notes:
- `claude-opus-5` runs adaptive thinking by default — good for extraction
  quality. Do **not** pass `budget_tokens`; it is removed on this model and
  returns a 400.
- Tune `output_config.effort` down to `"low"` on the update path if latency
  bites. `"medium"` is the starting point.
- **Use structured outputs** (`output_config.format` with a JSON Schema matching
  the `Context` struct) rather than asking for markdown and parsing it. The
  response deserializes straight into the Rust type — no regex, no
  malformed-section handling. This is precisely why §6 stores JSON.

### Prompt — Create

```
Given this AI conversation, extract a durable developer context that another
AI can use to continue the work.

Do not invent information.

Prioritize:
- goal
- current state
- decisions
- constraints
- things already tried
- relevant files
- errors
- open issues
- next steps

Remove conversational noise.
```

### Prompt — Update

```
Here is the existing context.
Here is a new conversation.

Update the existing context using information from the new conversation.
Preserve valid information.
Update outdated information.
Resolve contradictions in favor of newer explicit information.
Do not invent facts.
```

### Prompt — Handoff

```
Transform this context into a concise prompt that allows another AI model to
continue the work without restarting.
Assume the user wants to continue from the current state.
Do not repeat unnecessary background.
```

Write these as three plain functions. **Do not build a provider abstraction** —
add one only when a second provider is actually needed.

---

## 11. Build plan

### ✅ Milestone 0 — Scaffold (DONE)

Project created, all dependencies resolved, Rust + frontend both compile,
`pnpm tauri dev` runs. See "Current state" below.

### Milestone 1 — App shell (in progress)

Goal: a launcher that appears in under ~100ms and feels native. **No AI, no
database.** Ship this to yourself and live on the hotkey for a day before
touching anything else.

- [x] Tray / menu bar icon with Open + Quit
- [x] Global shortcut (`⌘⇧Space` / `Ctrl⇧Space`) toggling the window
- [x] Dock-less on macOS (`ActivationPolicy::Accessory`), `skipTaskbar` on Windows
- [x] Single-instance guard
- [x] `cmdk` launcher UI on fake data, ↑↓/↵ navigation, Escape to dismiss
- [x] **Hide on blur** — panel-delegate resign-key on macOS, `Focused(false)` elsewhere
- [x] **macOS `NSPanel` conversion** — non-activating panel via `tauri-nspanel`, pinned rev
- [ ] User-configurable shortcut, persisted
- [ ] Measure and defend show latency

### Milestone 2 — Local context storage

Implement create / save / edit / delete / search / copy against real SQLite. No
AI yet. **The app should be genuinely useful with hand-written contexts at the
end of this milestone** — that is the checkpoint.

- [x] `rusqlite` connection in Tauri managed state, migrations on startup
- [x] `Context` struct + serde, JSON into `contexts.content`
- [x] FTS5 virtual table + sync triggers
- [x] Tauri commands: `list_contexts`, `search_contexts`, `get_context`,
      `save_context`, `delete_context`, `delete_all_data`, `copy_context`
- [x] `copy_context` renders the struct to markdown in Rust and returns it
- [x] Replace `FAKE_CONTEXTS` in `src/App.tsx` with real `invoke` calls
- [x] Context detail view (Flow D) with Copy / Edit / Delete
- [x] Main browsing window (separate from the launcher panel)
- [ ] **Update** action on a context — deferred to Milestone 3, since it means
      "paste a newer conversation and re-derive", which needs the LLM

### Milestone 3 — AI generation

- [x] `ai.rs` — `reqwest` client, structured outputs, three prompts
- [x] `create_context_from_conversation` (Create prompt)
- [x] `update_context_from_conversation` (Update prompt)
- [x] `generate_handoff` (Handoff prompt)
- [x] Paste-and-generate UI with loading state, replacing the field form as the
      entry point
- [x] Raw conversation saved to `sources` (never to logs)
- [x] API proxy (`proxy/worker.js`) holding the key, with Gemini + Anthropic
      adapters and translation tests
- [ ] **Deploy the proxy and set `BATON_API_BASE`** — nothing works until this
      exists; the default URL is a placeholder. Steps in §14c.
- [ ] **Create the `RATE_LIMIT` KV namespace** — without it the daily cap is not
      enforced and spend is unbounded (§14c)
- [ ] Update §14 wording: conversations now transit our infrastructure

**Provider choice lives in the proxy, not the app.** Baton speaks the
Anthropic Messages format; the Worker translates to Gemini (default,
`gemini-3.7-flash`) or Anthropic. A desktop app cannot change providers without
a rebuild-and-redistribute cycle, so the switch belongs server-side. This is
the "add an abstraction only when multiple providers are actually needed"
point from §11 — it arrived, and the cheapest place to put it was the proxy.

**Key handling changed from the original PRD.** There is no user-supplied API
key and no settings pane. The app calls a proxy we operate, which injects the
key. An embedded key would be a public key — `strings` on the binary or a
debugging proxy recovers it immediately, and the bill is unbounded. The
tradeoff is that we now run infrastructure and handle other people's
conversations; see `proxy/README.md`.

### Milestone 4 — Handoff polish

- [ ] `generate_handoff` command (Handoff prompt)
- [ ] End-to-end flow: shortcut → create handoff → paste → generate → copy
- [ ] **Auto-copy on generation completion** so the next keystroke is ⌘V
- [ ] Instrument every non-LLM segment; hold each well under 100ms
- [ ] Per-platform modifier labels driven by `src/lib/platform.ts`
- [ ] Delete-context and delete-all-data actions surfaced in the UI

The handoff loop must feel extremely fast before adding any integrations.

---

## 12. v0 scope

**Must have:** desktop app · tray/menu bar presence · global shortcut · launcher
· create context · paste conversation · LLM context extraction · local
persistence · search · view · edit · copy to clipboard · update from new
conversation · API key configuration · delete context.

**Nice to have:** markdown rendering · recent contexts · context pinning ·
keyboard navigation · multiple LLM providers.

**Explicitly don't build:** browser extensions · chat integrations · IDE plugins
· cloud sync · team accounts · vector search · automatic conversation capture ·
Git integration · mobile app.

---

## 13. Validation

Test with 10–15 developers who regularly use multiple AI tools. **Do not ask
whether they like the idea.** Give them the product and observe what they do.

**Primary signal:** developers independently use the flow multiple times per
week when switching AI tools.

```
AI session → ⌘⇧Space → save/update context → new AI → paste
```

If this behavior repeats naturally, invest in the larger vision: automatic
conversation capture, project contexts, Git state, IDE integration, semantic
search, cross-device sync.

---

## 14. Decisions made during scaffolding

| # | Decision | Rationale |
|---|---|---|
| 1 | **Tauri v2, not Swift/SwiftUI** | One codebase for macOS + Windows. Cost: window-show latency is now our problem, not the OS's. |
| 2 | **`contexts.content` holds JSON, not markdown** | Resolves the docx §6/§15 contradiction. Markdown is rendered only at the clipboard boundary. |
| 3 | **Rust owns the DB, the API key, and all LLM calls** | The webview never sees a secret or a raw conversation. Required by §9. |
| 4 | **FTS5 instead of `LIKE`** | Same effort, ranked results, still not embeddings. |
| 5 | **`keyring` crate** | Single API over macOS Keychain and Windows Credential Manager. |
| 6 | **Window created once, hidden; only show/hide** | Creating per-invocation costs a 300–800ms webview boot. This is the whole ballgame for launcher feel. |
| 7 | **`macos-private-api` enabled** | Required for the transparent floating panel. **Consequence: cannot ship on the Mac App Store.** Direct distribution only — which is what Raycast-likes do anyway. |
| 8 | **Platform read once into `src/lib/platform.ts`** | Hardcoding ⌘ in JSX is the mistake that makes a port feel unported. |
| 9 | **No provider abstraction** | Per the original PRD. Add one only when a second provider actually exists. |

---

## 14b. Open decision — raw conversation storage

`sources` now holds unfiltered pasted conversations, indefinitely, in plaintext,
inside a Time Machine backup. Developers paste API keys, tokens and internal
URLs into AI chats constantly, so this is a materially larger exposure than the
structured contexts beside it.

Three options, cheapest first:

1. **Don't persist raw sources** — extract, then discard. Most consistent with
   §9's "raw conversations should not be logged".
2. **Expire them** — keep N days, then drop. Bounds the window.
3. **Encrypt the database** — SQLCipher with the key in the OS keystore, which
   is what Raycast does. `keyring` is already a dependency, so key storage is
   solved; the work is swapping the SQLite driver.

Currently option 0: kept forever, unencrypted. Decide before any real
distribution.

## 14c. Hosting — the API proxy

Generation does not work until the proxy is deployed. Until then
`BATON_API_BASE` points at a placeholder that does not resolve.

### Why a proxy exists at all

The app ships with no API key. Anything inside a distributed binary is public —
`strings` on the app, or a debugging proxy on the wire, recovers an embedded key
in seconds, and the resulting bill has no ceiling. The key lives on a server we
control; the app only ever talks to that server.

### Why provider choice lives there too

Baton always speaks the **Anthropic Messages** wire format. The Worker
translates to Gemini (default) or Anthropic based on its `PROVIDER` var.

A desktop app cannot change providers without rebuilding, re-signing, and
getting every user to update. In the Worker it is one variable and a deploy —
which also makes provider fallback possible without touching clients. New
providers are added as adapters in `proxy/worker.js`, never as branching in
`ai.rs`.

### Deploy

`proxy/` is a complete Worker project — there is nothing else to scaffold.

```bash
cd proxy
npx wrangler login                       # opens a browser; needs a Cloudflare account
npx wrangler secret put GEMINI_API_KEY   # paste the key; stored by Cloudflare, not in git
npx wrangler deploy                      # prints the live URL
```

Then rebuild the app against the printed URL:

```bash
BATON_API_BASE=https://baton-proxy.<subdomain>.workers.dev pnpm tauri build
```

Once a real domain exists, change the default in `src-tauri/src/ai.rs` so plain
`pnpm tauri build` is correct without the env var.

### Rate limiting (do this before sharing the app)

The daily cap is the only thing bounding spend, and it needs a KV namespace:

```bash
npx wrangler kv namespace create RATE_LIMIT
```

Paste the printed id into the commented `[[kv_namespaces]]` block in
`proxy/wrangler.toml`, uncomment it, and redeploy. **Without KV the Worker still
runs but enforces no cap.**

### Configuration

| Setting | Where | Notes |
|---|---|---|
| `PROVIDER` | `wrangler.toml` `[vars]` | `gemini` (default) or `anthropic` |
| `MODEL` | `wrangler.toml` `[vars]` | optional; defaults `gemini-3.7-flash` / `claude-opus-5` |
| `GEMINI_API_KEY` / `ANTHROPIC_API_KEY` | `wrangler secret put` | never in git |
| `RATE_LIMIT` | KV binding | omit and the cap is not enforced |

### What the Worker enforces

`POST /v1/messages` only · model pinned server-side (callers cannot choose the
model you pay for) · 1MB body cap · 100 requests per device per day.

The device id is client-generated and spoofable — friction against casual
abuse, not authentication. If the endpoint leaks publicly, move to signed tokens
or accounts.

### Tests

```bash
node proxy/worker.test.mjs
```

Stubs `fetch` and asserts the request sent upstream and the response returned to
Baton, including split text blocks, blocked responses becoming
`stop_reason: "refusal"`, empty output failing loudly rather than saving an
empty context, and the rate limit.

### Cost

| Model | Typical extraction | Full-session dump |
|---|---|---|
| `gemini-3.7-flash` | fractions of a cent | a few cents |
| `claude-opus-5` | ~$0.12 | ~$0.50 |

Extraction is compression, not hard reasoning, which is why a Flash-tier model
is the default. Confirm current rates before budgeting.

## 15. Known gaps / risks

1. **macOS `NSPanel` conversion — DONE.** The window is swizzled into a
   non-activating `NSPanel` via `tauri-nspanel` (pinned to rev `18ffb9a2`).
   Style mask `NonActivatingPanel`, floating level, collection behaviour
   `canJoinAllSpaces | fullScreenAuxiliary`, `hidesOnDeactivate` off. This is
   what lets the launcher appear over fullscreen apps without switching Spaces:
   a regular NSWindow cannot become key over another app's fullscreen Space.
   Consequence: the crate replaces tao's window delegate, so Tauri
   `WindowEvent::Focused` never fires on macOS — dismissal is handled by the
   panel delegate's `window_did_resign_key` instead. Windows needs none of
   this and keeps the plain window path.

2. **Code signing, twice.** Apple Developer account ($99/yr) for macOS
   notarization; a Windows code-signing certificate or users get SmartScreen
   warnings. Not blocking development, blocking distribution.

3. **Window-show latency is now our responsibility.** Raycast is native on both
   platforms; a webview launcher has to actively defend this. The
   create-once/show-hide pattern handles it, but hold the line.

4. **Extraction quality is the real product risk**, not speed. See §2.

---

## 16. Current state (as of scaffold)

```
baton/
├── PRD.md                    ← this file
├── CLAUDE.md                 ← context for AI coding sessions
├── package.json
├── pnpm-workspace.yaml       ← allowBuilds: esbuild (pnpm 11 requirement)
├── vite.config.ts            ← react + tailwind v4
├── src/
│   ├── App.tsx               ← cmdk launcher, FAKE DATA
│   ├── main.tsx
│   ├── index.css             ← tailwind v4 entry, transparent body
│   ├── types.ts              ← Context interface (mirrors Rust)
│   └── lib/platform.ts       ← IS_MAC, MOD_LABEL, hasMod()
└── src-tauri/
    ├── tauri.conf.json       ← hidden/transparent/undecorated/always-on-top
    ├── Cargo.toml
    ├── capabilities/default.json
    └── src/
        ├── lib.rs            ← plugins, shortcut, activation policy, setup
        ├── launcher.rs       ← show/hide/toggle
        ├── tray.rs           ← menu bar / tray icon
        └── main.rs
```

**Verified working:** `pnpm build` (frontend) and `cargo build` (Rust) both
succeed; `pnpm tauri dev` launches the app.

**Not yet built:** everything in Milestones 2–4, plus the Milestone 1 unchecked
boxes above.

### Run it

```bash
source "$HOME/.cargo/env"   # if cargo is not on PATH
cd "path/to/baton"
pnpm install
pnpm tauri dev
```

The window starts hidden. Press `⌘⇧Space` (macOS) or `Ctrl⇧Space` (Windows), or
use the tray icon.
