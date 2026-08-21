# Baton — notes for AI coding sessions

**Read `PRD.md` first.** It is the source of truth for what this product is,
what is deliberately excluded, and what the next milestone is. This file only
covers how to work in the repo.

## What this is

**Baton** is a local-first, cross-platform (macOS + Windows) Raycast-style
launcher that stores durable *context* documents about what a developer is
working on, and copies them to the clipboard so they can be pasted into any AI
tool. Tauri v2 + React frontend + Rust core.

Note the vocabulary split: **Baton** is the product; a **context** is the thing
it stores (`Context` struct, `contexts` table, `list_contexts` command). Keep
them distinct in code and copy.

## Commands

```bash
source "$HOME/.cargo/env"   # cargo was installed via rustup; may not be on PATH
pnpm install
pnpm tauri dev              # run the app
pnpm build                  # frontend typecheck + build
cd src-tauri && cargo build  # Rust only
```

The window starts **hidden**. Summon with `⌘⇧Space` (macOS) / `Ctrl⇧Space`
(Windows), or the tray icon.

## Architecture rules — do not violate these

1. **Rust owns everything sensitive.** The API key, the SQLite connection, and
   all Anthropic HTTP calls live in `src-tauri/`. The webview must never
   receive an API key or hold a raw conversation longer than the `invoke` that
   submits it. This is a privacy requirement, not a style preference —
   see PRD §9.

2. **`contexts.content` stores JSON, not markdown.** The `Context` struct is
   the source of truth; markdown is rendered on demand at the clipboard
   boundary only. The original PRD contradicted itself here; PRD §6 has the
   resolution.

3. **Never log raw conversations.** No `dbg!`, no `println!`, and no error
   messages that embed request bodies.

4. **The launcher window is created once at startup, hidden, and only ever
   shown/hidden.** Never create or destroy it per invocation — that is a
   300–800ms webview boot and it is the single thing that would make this feel
   like a web app instead of a launcher. See `src-tauri/src/launcher.rs`.

5. **No provider abstraction.** One LLM provider (Anthropic) until a second is
   genuinely needed.

6. **Never hardcode `⌘` in JSX.** Import from `src/lib/platform.ts`.

## Anthropic API

No official Rust SDK — raw `reqwest` in `src-tauri/src/ai.rs`.

**The app has no API key.** It POSTs to a proxy we run (`proxy/worker.js`),
which injects the key. Never embed a key in the binary: `strings` or a wire
proxy recovers it instantly. `BATON_API_BASE` overrides the endpoint at build
time.

**The app is provider-agnostic and must stay that way.** Baton always speaks
the Anthropic Messages format; the Worker translates to Gemini (the default) or
Anthropic based on its `PROVIDER` var. Do not add provider branching to
`ai.rs` — Baton is a desktop app, so a client-side switch means rebuilding and
redistributing binaries, while a Worker switch is a deploy. Add new providers
as adapters in `proxy/worker.js`, covered by `proxy/worker.test.mjs`.

- Model: `claude-opus-5`
- Headers: `x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`
- Adaptive thinking is on by default. **Do not pass `budget_tokens`** — it is
  removed on this model and returns a 400.
- Tune with `output_config.effort` (`"low"` | `"medium"` | `"high"`), not token budgets.
- Prefer **structured outputs** (`output_config.format` + JSON Schema matching
  the `Context` struct) over parsing markdown out of prose.

Full prompt text for create/update/handoff is in PRD §10.

## Layout

```
src/                     React webview — pure UI, no secrets
  App.tsx                routes on ?view=browser
  Launcher.tsx           cmdk launcher panel
  Browser.tsx            main browsing window (PRD §13)
  components/
    ContextDetail.tsx    Flow D: view / edit / copy / delete
  lib/api.ts             every invoke() call lives here, nowhere else
  lib/platform.ts        IS_MAC, MOD_LABEL, hasMod()
  types.ts               Context interface, mirrors the Rust struct
src-tauri/src/
  lib.rs                 plugin registration, global shortcut, setup hook
  commands.rs            the whole IPC surface
  db.rs                  SQLite: schema, migrations, FTS5, queries
  context.rs             Context type + markdown rendering
  launcher.rs            show / hide / toggle, NSPanel, vibrancy
  tray.rs                menu bar / system tray
```

## Storage

`~/Library/Application Support/com.aviralj02.baton/baton.sqlite3` on macOS
(app-data dir keyed by bundle identifier). Schema version lives in
`PRAGMA user_version`; add a numbered block in `db::migrate` to change it —
never edit an existing block, it has already run on real data.

`cargo test --lib` covers the storage layer (12 tests). The FTS index is
*external-content*: it stores no copy of the rows and is kept in sync purely by
the three triggers. If you touch the `contexts` schema, verify
`fts_index_stays_in_sync_on_update_and_delete` still passes — a broken trigger
makes search silently return stale results rather than fail.

## Gotchas already hit

- **pnpm 11** uses `allowBuilds:` in `pnpm-workspace.yaml` to approve build
  scripts, not `onlyBuiltDependencies` and not the `pnpm` field in
  `package.json`. `esbuild` is already approved there; if you add a dep with a
  postinstall script you will need to add it too.
- **`macOSPrivateApi: true`** in `tauri.conf.json` requires the
  `macos-private-api` cargo feature on the `tauri` crate. They must be changed
  together or the build script fails with an unhelpful allowlist error.
- **reqwest 0.13** renamed the TLS feature; `rustls-tls` does not exist. rustls
  is on by default — only `json` needs to be requested.
- **keyring 4.x** enables macOS Keychain and Windows Credential Manager through
  its default `v1` feature. Do not pass `apple-native` / `windows-native`.
- **AppKit is main-thread-only.** The global-shortcut handler and the
  single-instance callback run on tokio worker threads; touching NSWindow from
  them crashes with EXC_BREAKPOINT (this happened). Tauri's own window methods
  marshal internally; anything hand-rolled must go through
  `run_on_main_thread`. The nspanel handle is not `Send` — fetch it *inside*
  the main-thread closure via `get_webview_panel`, don't move it in.
- **tauri-nspanel replaces tao's window delegate on macOS**, so
  `WindowEvent::Focused` never fires there. Blur-dismissal lives in the panel
  delegate (`launcher.rs`); the `on_window_event` handler in `lib.rs` is for
  Windows. The crate is pinned by rev in Cargo.toml — bump deliberately.
- **The structured-output JSON Schema must match `ContextBody`'s serde names
  exactly** (`ai.rs`). A mismatch is not an error — the model returns valid
  JSON, serde fills defaults, and the context saves silently empty. Pinned by
  `schema_field_names_match_the_struct`.
- **`stop_reason: "refusal"` arrives as HTTP 200.** Check it before reading
  `content`, or a declined request looks like an empty response.
- **Never hold the DB `Mutex` across an `.await`.** The AI commands take the
  lock, drop it, make the request, then re-take it — holding it would block
  every other command for the duration of the API call.
- **Launcher dismissal needs a blur grace period** (400ms, `launcher.rs`):
  a resign-key/blur arriving right after show() is part of the show
  transition; hiding on it makes the launcher flash open and vanish.

## Immediate next task

Deploy the proxy (`proxy/README.md`) and set `BATON_API_BASE` — generation
cannot work until then. After that: PRD §14b (raw conversation storage), the
configurable shortcut, and a first Windows test pass.
