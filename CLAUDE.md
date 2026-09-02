# Baton — notes for AI coding sessions

**Read `docs/PLAN.md` first** for what this is, where it stands and what is next.
This file is only how to work in the repo.

## What it is, in one paragraph

A launcher over a markdown wiki at `~/Baton/`. An agent files what a session learned
via the `/baton` skill; Baton indexes those files and one hotkey puts a whole
project's context on the clipboard. **Baton makes no model calls** — no API key, no
network code. The files are the source of truth; SQLite is a rebuildable index.

Tauri v2 + React frontend + Rust core.

## Commands

```bash
pnpm install
pnpm tauri dev               # run the app
pnpm build                   # frontend typecheck + build
cd src-tauri && cargo test   # 98 tests
```

The window starts **hidden**. Summon with `⌘⇧Space` / `Ctrl⇧Space`, or the tray icon.
If `cargo` is missing, `source "$HOME/.cargo/env"`.

## Vocabulary

The UI and the code must agree on this.

| On disk | In code | To the user |
|---|---|---|
| `concepts/` | `PageType::Gotcha` | **constraint** |
| `projects/<slug>/` | `project` | **project** |

The launcher shows **projects only**. The per-type file split is how the wiki
organises itself, not a choice to put in front of someone mid-paste.

## Rules — do not violate these

1. **Files are the source of truth.** The SQLite tables are derived and may be deleted
   and rebuilt at any time. Never let a fact live only in the database.

2. **Baton does not write pages.** The agent writes them through the skill. There is
   deliberately no `write_page` command — a second write path is a second place for
   the schema to be violated. Baton *does* own the derived files: `index.md` and the
   index itself, and it may **delete** pages: removal cannot violate a schema, and a
   wiki you can only add to is a hoard. Deletion goes to the OS trash, never
   `remove_file` — see `remove.rs`.

3. **`AGENTS.md` and `lint.rs` are one contract in two languages.** Required sections,
   allowed headings and status rules are hard-coded in `lint.rs`. Change one, change
   the other — a false lint finding is worse than a missing check, because findings
   are pasted into the primer and believed.

4. **The launcher window is created once at startup, hidden, and only ever
   shown/hidden.** Creating it per invocation is a 300–800ms webview boot and the one
   thing that would make this feel like a web app.

5. **`skills/` is the only home for the schema and the command.** `onboarding.rs`
   embeds both with `include_str!`.

6. **A shortcut is rendered by `components/Shortcut.tsx`, never spelled out.** Give
   it the accelerator `get_shortcut` returns and it draws one keycap per key,
   falling back to `DEFAULT_SHORTCUT` from `lib/platform.ts` only until that call
   answers. Nothing else may build a shortcut label: `IS_MAC` decides whether a
   modifier is a drawn symbol or the word the key carries on that platform, and it
   decides it in one file.

7. **A survivable failure is a `notice::report`, not an `eprintln!`.** A packaged app
   has no stderr, so anything the user could act on has to reach a window. Startup
   runs before any webview exists, which is why notices queue.

8. **Colour and type come from `src/index.css`, and nothing else.** Components use
   the semantic utilities (`bg-surface`, `text-body`, `border-line`, `bg-brand`,
   `text-ui`) and carry **no `dark:` variants**: the tokens themselves flip under
   `prefers-color-scheme`, so a surface is written once. No `stone-400`, no hex, no
   `text-[13px]`. If a value is missing, add a token here first. Anything filled
   with a colour that does not invert pairs with its own `on-*` token, which is why
   `--on-brand` and `--on-danger` exist. Repeated class strings become a primitive
   in `components/`, not a fourth copy at a call site.

   `site/src/index.css` repeats the colour half of this by hand, because the two
   builds share no code. The hex values are the same on both sides and must stay
   that way. The type scale is deliberately not shared: the app is read at a
   glance, the page at arm's length, so the same names carry larger sizes there.

9. **No character ever stands in for an icon.** Every mark in the UI is an SVG from
   `components/Icon.tsx`, drawn on the same 16px grid at the same stroke weight, so
   a new one cannot arrive at a different weight than its neighbours. `↑ ↓ → ← ✓ ●
   · • ›` and friends are out, in JSX and in copy alike: they carry a different
   weight than the icon set, they drift off the baseline, and they inherit no size
   scale. An icon is `aria-hidden`, so anything conveying meaning on its own needs
   a label from its container, which is what `Key`'s `label` prop is for.

   Modifier keys included. `CommandIcon`, `ShiftIcon`, `ControlIcon` and
   `OptionIcon` are paths on that same grid, because the characters arrive at the
   mono face's weight, sit off a keycap's optical centre, and ignore the size
   scale. `Shortcut` is the only thing that renders them.

   One exception, and only because the medium cannot hold anything else: a native
   `title` attribute or an `<option>` takes ordinary punctuation.

   `grep -rn "[⌘⌃⌥⇧›→←↑↓↻↗·•★✓✗]" src/ site/src/` should return nothing but the
   doc comments in `Icon.tsx` and `Shortcut.tsx` that name the characters in order
   to rule them out.

## Layout

```
src/                     React webview
  index.css              the token layer: every colour and type size in the app
  Launcher.tsx           projects only; ↵ copies a project's whole context
  Browser.tsx            page browsing, grouped per project
  components/
    PageDetail.tsx       one page, with backlinks
    Setup.tsx            first run: wiki path, install the skill
    Button.tsx           every text button and icon button
    Label.tsx            the caps group label and the serif section heading
    Shortcut.tsx         an accelerator as one drawn keycap per key
  lib/api.ts             every invoke() call lives here, nowhere else
src-tauri/src/
  commands.rs            the whole IPC surface
  wiki.rs                read/parse a page: frontmatter, sections, links
  db.rs                  the index: schema, migrations, FTS5, queries
  primer.rs              assemble one project's pages into a brief
  lint.rs                structural checks, surfaced in the primer
  index_md.rs            regenerate ~/Baton/index.md from the tree
  remove.rs              delete a page, a project or the lot — to the trash
  settings.rs            the summon shortcut, persisted beside the index
  notice.rs              problems the user needs to hear, queued until a window exists
  update.rs              check for and install a newer Baton
  watcher.rs             reindex on change, debounced
  onboarding.rs          create the wiki, install the skill
  launcher.rs            show / hide / toggle, NSPanel, vibrancy
skills/                  the schema and the command: embedded and installable
site/                    the download page: its own Vite build, deployed separately
```

Storage: `~/Library/Application Support/com.aviralj02.baton/baton.sqlite3`. Four
tables — `pages`, `sections`, `links`, `pages_fts` — all derived from the markdown.
There are no migrations: edit the `SCHEMA` constant in `db.rs` and the next launch
drops the index and rebuilds it from the files. `ensure_schema` also drops any table
`SCHEMA` no longer creates, so there is no second list to keep in step.

## Gotchas

- **AppKit is main-thread-only.** The global-shortcut handler and the single-instance
  callback run on tokio worker threads; touching NSWindow from them crashes with
  `EXC_BREAKPOINT`. Tauri's own window methods marshal internally; anything
  hand-rolled must go through `run_on_main_thread`. The nspanel handle is not `Send` —
  fetch it *inside* the closure, don't move it in.
- **tauri-nspanel replaces tao's window delegate on macOS**, so `WindowEvent::Focused`
  never fires there. Blur-dismissal lives in the panel delegate; the `on_window_event`
  handler is for Windows. The crate is pinned by rev — bump deliberately.
- **Launcher dismissal needs a blur grace period** (400ms). A resign-key arriving
  right after `show()` is part of the show transition; hiding on it makes the launcher
  flash open and vanish.
- **A derived file inside a watched folder loops.** `index.md`, `log.md` and
  `AGENTS.md` are excluded from the watch filter, and `write_if_changed` skips an
  identical write. Either guard alone leaves a reindex that never settles.
- **`db::sync` is incremental** and never holds the whole tree, so `index_md` walks
  the folder itself. Reusing the indexer's output lists only the changed pages.
- **Code behind a false `#[cfg]` is not type-checked.** CI compiles the non-macOS
  branches on `windows-latest`, so they are at least type-checked now. Nobody has
  ever *run* them: "it builds there" is not "it works there".
- **`macOSPrivateApi: true`** in `tauri.conf.json` requires the `macos-private-api`
  cargo feature. Change both together or the build fails with an unhelpful allowlist
  error.
- **pnpm 11** approves build scripts via `allowBuilds:` in `pnpm-workspace.yaml`, not
  `onlyBuiltDependencies` and not the `pnpm` field in `package.json`.
- **A missing wiki root is an empty one, not an error.** `wiki::walk` fails on a
  deleted directory, and if `db::sync` propagates that it returns before
  `remove_missing` runs — so every row for a vanished file survives and no later
  sweep clears it. Every reindex path also calls `ensure_wiki`, so deleting the
  folder gets you a fresh one rather than an app that quietly does nothing.
- **Never hold the DB `Mutex` across a file read.**
