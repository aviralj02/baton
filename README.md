# Baton

A local-first context layer for AI-assisted development. Capture what you're
working on once, then hand it off to any AI tool — Claude, ChatGPT, Codex,
Cursor, Gemini — without re-explaining yourself.

*Your context, independent of the AI you're using.*

**Platforms:** macOS + Windows · **Stack:** Tauri v2 + React + Rust

## Getting started

```bash
pnpm install
pnpm tauri dev
```

The window starts hidden. Summon it with `⌘⇧Space` (macOS) or `Ctrl⇧Space`
(Windows), or from the tray icon.

> If `cargo` isn't found, run `source "$HOME/.cargo/env"` first, or add it to
> your shell profile.

## Docs

- **[PRD.md](PRD.md)** — product spec, architecture decisions, milestone plan
- **[CLAUDE.md](CLAUDE.md)** — architecture rules and gotchas for AI coding sessions
