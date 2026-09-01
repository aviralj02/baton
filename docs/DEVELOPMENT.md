# Development

How to build Baton and how the pieces fit. For what it is and why, see the
[README](../README.md); for what is next, [PLAN.md](PLAN.md); for the rules an
AI coding session must follow, [CLAUDE.md](../CLAUDE.md).

## Prerequisites

[Node 20+](https://nodejs.org), [pnpm](https://pnpm.io), and
[Rust](https://rustup.rs). macOS also needs the Xcode Command Line Tools
(`xcode-select --install`) — the full Xcode app is not required.

## Commands

```bash
pnpm install
pnpm install-baton           # install /baton, create ~/Baton
pnpm tauri dev               # run the app
pnpm build                   # frontend typecheck + build
cd src-tauri && cargo test   # 98 tests
cd src-tauri && cargo clippy --all-targets -- -D warnings
pnpm format:check            # prettier, also gated in CI
```

The clippy line is the same gate CI runs, so run it before pushing or the build
goes red on style.

If `cargo` is not found, run `source "$HOME/.cargo/env"` or add it to your shell
profile.

The window starts **hidden**. Summon with `⌘⇧Space` / `Ctrl⇧Space`, or the tray
icon.

## What the installer does

`pnpm install-baton` is what makes the `/baton` command exist. It copies
`skills/baton/SKILL.md` into the skills directory of every agent tool it finds
(`~/.claude`, `~/.codex`, `~/.cursor`), creates `~/Baton/` if missing, and seeds
`~/Baton/AGENTS.md` with the page schema.

It installs at user level rather than committing to `.claude/skills/` in this
repo, because you run `/baton` in whatever project the session was about, not in
this one.

Two things to know:

- **`skills/` in this repo is canonical.** The installer overwrites the copies in
  your home directory on every run, so editing one of those loses the edit.
- **`~/Baton/AGENTS.md` is never overwritten.** It is the contract every existing
  page was written against and is meant to be edited as the schema is repaired.
  Delete it if you want the installer to reseed it.

`git init ~/Baton` is worth doing for history and undo. It needs no remote.

## How it fits together

```
skills/                  the schema and the /baton command, embedded and installable
~/Baton/*.md             the wiki: the source of truth, outside the repo
src-tauri/               Rust core: index, primer, lint, watcher, window
src/                     React webview: launcher and browser windows
```

**Files are the source of truth.** SQLite is a derived index that can be deleted
at any time; the next launch rebuilds it from the folder. There are no schema
migrations — `db.rs` holds one `SCHEMA` constant, fingerprinted, and the index is
rebuilt whenever it stops matching the code.

**Baton does not write pages.** The agent does, through the skill. There is
deliberately no `write_page` command: a second write path is a second place for
the schema to be violated.

**`AGENTS.md` and `lint.rs` are one contract in two languages.** Required
sections, allowed headings and status rules are hard-coded in the linter. Change
one and change the other — a false lint finding is worse than a missing check,
because findings are pasted into the primer and believed.

Full architecture rules and the gotchas that cost time are in
[CLAUDE.md](../CLAUDE.md).

## Testing

`cargo test --lib` covers the wiki parser, the index, the primer, lint, index
regeneration, the watcher and onboarding. `pnpm build` typechecks the frontend.

When splitting work across commits, verify each revision from a **clean
checkout**, not the working tree:

```bash
git worktree add --detach /tmp/verify <rev>
cd /tmp/verify && cargo test
```

Testing in place proves nothing about what was staged. Two commits in one
session were broken this way and neither was caught until the method changed.

### The download page

`site/` is a separate Vite app, a workspace member rather than part of the
Tauri frontend. `pnpm --filter baton-site dev` runs it.

It resolves downloads at page load from the GitHub releases API, matching assets
by extension rather than by name, because Tauri puts the version in the filename
and a hardcoded link dies at the next release. Every button falls back to the
releases page when that lookup fails, which it will while nothing is published.

Deployed by Vercel, not from this repo's workflows. The project's **Root
Directory** is `site`, with "include source files outside the root directory"
enabled so the workspace lockfile resolves. Vercel detects Vite and pnpm on its
own; `dist` is the output.

It builds from the domain root, which is why `vite.config.ts` sets no `base`.
Moving it to a subpath means setting one to match.

### CI

`.github/workflows/ci.yml` runs the frontend typecheck on Linux, and
`cargo check --all-targets`, `cargo test --lib` and clippy on **macOS and
Windows**. The Windows job is the point: it is the only thing that type-checks
the non-macOS `#[cfg]` branches, which nobody has ever compiled otherwise. It
stubs `dist/index.html` so it does not depend on the frontend job.

### Releasing

Bump the version, push a `v*` tag, publish the draft that appears. The steps are
in **[RELEASE.md](RELEASE.md)**.

A local `pnpm tauri build` now needs the updater signing key, because
`createUpdaterArtifacts` is on and Tauri refuses to emit an unsigned manifest:

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/baton-updater.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
pnpm tauri build
```

`pnpm tauri dev` is unaffected. CI takes the same values from repository
secrets.

## Not yet done

- [ ] No code signing. This is stronger than a warning on macOS: Gatekeeper refuses to open an unsigned download at all. Needs an Apple Developer account; the release workflow is already wired for the secrets.
