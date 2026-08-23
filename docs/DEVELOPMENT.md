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
cd src-tauri && cargo test   # 84 tests
```

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

## Not yet done

- [ ] No code signing, so a built app triggers Gatekeeper and SmartScreen warnings.
- [ ] Windows has never had a real test pass — the platform branches are behind `#[cfg]`, and code behind a false `cfg` is not type-checked, so "it builds on macOS" says nothing about them. See [PLAN.md](PLAN.md).
- [ ] Delete all data button in Browser like Rebuild Index one.
- [ ] Delete a page or whole context button also