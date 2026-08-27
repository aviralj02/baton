<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <img src="assets/logo-light.svg" alt="Baton" width="76" height="76">
  </picture>
</p>

<h1 align="center">Baton</h1>

<p align="center">
  <b>Your context, independent of the AI you're using.</b><br>
  A local-first memory layer for AI-assisted development.
</p>

<p align="center">
  macOS &amp; Windows · Tauri + Rust · No account, no cloud, no API key
</p>

<!-- TODO on first release: uncomment, and point at the real URLs.
<p align="center">
  <a href="https://github.com/aviralj02/baton/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/aviralj02/baton"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/aviralj02/baton"></a>
  <a href="https://github.com/aviralj02/baton/actions"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/aviralj02/baton/ci.yml"></a>
</p>
-->

---

## 🧩 The problem

You spend an hour with an AI getting somewhere real. It learns your constraints,
what you ruled out, what already failed.

Then the chat ends. Open a new one — or a different tool — and you start from
nothing. So you re-explain the goal, re-paste the files, and watch it suggest
the exact approach you abandoned yesterday.

**The context died with the conversation. Baton keeps it.**

## ⚡ How it works

```
   work with your agent  ──▶  /baton  ──▶  ~/Baton/*.md
                                              │
   new chat, any tool    ◀──  ⌘⇧Space  ◀──────┘
```

**1. Your agent writes it.** Finish a session, run `/baton`. The agent that did
the work files what it learned as markdown — decisions with their rejected
alternatives, approaches that failed, constraints that bit you.

**2. Baton indexes it.** Plain files in `~/Baton/`. You own them, you can edit
them, they work with git.

**3. One key gets it back.** Press `⌘⇧Space`, pick a project, hit enter. Its
entire context is on your clipboard. Paste into Claude, ChatGPT, Cursor, Codex,
anything.

Baton **never calls a model.** It has no API key and makes no network requests.
Your agent does the writing; Baton does the remembering.

## 🚀 Get started

### Download

**Coming soon.** Signed builds for macOS and Windows are not out yet.

<!-- TODO on first release: replace the line above with this.
| macOS | Windows |
|---|---|
| [Download .dmg](TODO) — Intel and Apple silicon | [Download installer](TODO) |

Then run `/baton` once in any project to finish setup.
-->

### Build from source

Works today and takes about a minute — see
**[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)**.

Once it is running, press `⌘⇧Space` (or `Ctrl⇧Space` on Windows) from anywhere.

## 📓 Daily use

| Action | How |
|---|---|
| File what a session learned | `/baton` in your agent |
| Get a project's context | `⌘⇧Space` → pick → `↵` |
| Read or edit a page | Open the main window, or just edit the markdown |

### 💡 Make it automatic

The one habit Baton depends on is running `/baton`. Put a line in your project's
agent instructions so it happens without you remembering:

```markdown
<!-- CLAUDE.md, AGENTS.md, or .cursor/rules -->
Run /baton after finishing a meaningful piece of work,
or every few commands during a long session.
```

Now every session leaves the next one better informed.

## 🔒 What it doesn't do

No accounts. No cloud sync. No telemetry. No model calls. Nothing leaves your
machine — the wiki is a folder you can read, grep, and delete.

## 📚 Docs

| | |
|---|---|
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Build it, hack on it, how the pieces fit |
| [docs/PLAN.md](docs/PLAN.md) | Phases, decisions, what's next |
| [docs/RELEASE.md](docs/RELEASE.md) | How a release is cut |
| [CLAUDE.md](CLAUDE.md) | Rules and gotchas for AI coding sessions |

## 🛠️ Status

Working and used daily, but pre-release. The builds are not code-signed yet:
Windows shows a SmartScreen warning you can click through, and macOS refuses the
app until you approve it under System Settings → Privacy & Security. Windows
compiles in CI but has not had a real hands-on pass. Issues and PRs welcome.

<!-- TODO on first release: swap the paragraph above for a short one-liner and
     uncomment the badges at the top. -->

## 📄 License

[MIT](LICENSE) © Aviral Jain
