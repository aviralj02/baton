# Baton

A local-first context layer for AI-assisted development. Capture what you're
working on once, then hand it off to any AI tool — Claude, ChatGPT, Codex,
Cursor, Gemini — without re-explaining yourself.

*Your context, independent of the AI you're using.*

**Platforms:** macOS + Windows · **Stack:** Tauri v2 + React + Rust

## Getting started

```bash
pnpm install
pnpm install-baton     # installs the /baton command and creates ~/Baton
pnpm tauri dev
```

The window starts hidden. Summon it with `⌘⇧Space` (macOS) or `Ctrl⇧Space`
(Windows), or from the tray icon.

> If `cargo` isn't found, run `source "$HOME/.cargo/env"` first, or add it to
> your shell profile.

## How it works

Baton reads a wiki of markdown files at `~/Baton/`. The files are the source of
truth. SQLite is only a search index over them and can be deleted at any time.

Pages are written by the agent that did the work. At the end of a session you
run `/baton`, the agent proposes page edits, and you approve them. Baton itself
never calls a model.

`pnpm install-baton` is what makes that command exist. It copies
`skills/baton/SKILL.md` into the skills directory of every agent tool it finds
(`~/.claude`, `~/.codex`, `~/.cursor`), creates `~/Baton/` if it is missing, and
seeds `~/Baton/AGENTS.md` with the page schema.

The command has to be installed at user level rather than checked in at
`.claude/skills/`, because you run it in whatever repo the session was about,
not in this one.

Two things to know:

- **`skills/` in this repo is canonical.** The installer overwrites the copies
  in your home directory on every run, so editing one of those loses the edit.
- **`~/Baton/AGENTS.md` is never overwritten.** It is the contract every
  existing page was written against, and it is meant to be edited as the schema
  is repaired. Delete it if you want the installer to reseed it.

`git init ~/Baton` is worth doing for history and undo. It needs no remote.

## Docs

- **[docs/PLAN.md](docs/PLAN.md)** — the plan of record: phases, decisions, what is next
- **[CLAUDE.md](CLAUDE.md)** — architecture rules and gotchas for AI coding sessions
- **[docs/PLAN.md](docs/PLAN.md)** — the live phase plan and decision log
