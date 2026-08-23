# Baton build plan

**This is the only design document.** Update it in place as work lands; never start a
parallel status doc. Working rules and gotchas live in `CLAUDE.md`. The knowledge
itself lives in `~/Baton/` — `index.md` lists every page, `AGENTS.md` is the schema.
Trust those over any list here.

---

## What this is

Developers work across several AI tools, and context is trapped inside individual
conversations. Starting a new chat means re-summarising, re-explaining decisions, and
re-listing what was already tried.

A central folder of markdown at `~/Baton/` holds a wiki about every project. At the end
of a session the user runs `/baton`, and the agent that did the work files what it
learned. **Baton itself never calls a model.** It indexes the files, and one
hotkey puts a whole project's context on the clipboard.

Three parts: a skill file the agent runs, a folder of markdown, and a launcher that
reads it.

**Positioning:** Raycast puts a command a keystroke away; this puts what you know a
keystroke away.

**It works when** a fresh agent session needs no correction from you in its first few
exchanges. That is the bar for every phase.

---

## Where we are

| Phase | Delivers | Status |
|---|---|---|
| 0 | Validate the idea with no code | `done`, **cold test still unrun** |
| 0.5 | First run: create the wiki, offer the skill | `done` |
| 1 | The wiki store | `done` |
| 2 | The primer | `done` |
| 3 | Lint, surfaced in the primer | `done` |
| 4 | Prune what the wiki design made dead | `done` |

### Verified state

- `cargo test --lib`: 80 passing across `wiki`, `db`, `primer`, `lint`, `index_md`,
  `watcher`, `onboarding`. `cargo build` has no dead-code warnings. `pnpm build` clean.
- 15 commands, all project, page, primer or setup.
- Baton makes no model calls and contains no network code.
- The launcher lists **projects**, one row each; `↵` copies that project's whole
  context, every page in full. Search matches project names and page titles, never
  body text.
- The browser window browses pages, grouped per project, constraints last.
- Pages written while the app runs appear without a restart; `index.md` is regenerated
  on every reindex. Deleting `~/Baton/` recreates it and empties the index, whether the
  app is running or not.
- First run creates `~/Baton/` and offers to install `/baton`.

---

## What is left

**Before an MVP ships**

- [ ] Phase 0 acceptance test in a genuinely cold session.
- [ ] User-configurable global shortcut, persisted.
- [ ] Measure and defend launcher show latency. Hold well under 100ms.
- [ ] First real Windows pass. Fourteen `#[cfg]` branches have never been compiled;
      code behind a false `cfg` is not type-checked, so "it builds here" says nothing.
      A `windows-latest` CI job running `cargo check` is the cheap 80%.
- [ ] Code signing: an Apple Developer account for notarisation, and a Windows
      certificate. Blocks distribution, not development.

**Smaller, not blocking**

- [ ] The setup screen lives only in the browser window. Fine while the launcher is a
      panel; revisit if setup ever needs to interrupt a summon.

---

## The design, in the parts that constrain code

### Page types

| Type | Lives in | Required sections |
|---|---|---|
| `project` | `projects/<slug>/overview.md` | Goal, Current state, Next step |
| `decision` | `projects/<slug>/decisions/` | Decision, Why, Rejected |
| `open` | `projects/<slug>/open/` | The question, Options, What it blocks |
| `attempt` | `projects/<slug>/attempts/` | What was tried, Why it failed, What it cost |
| `component` | `projects/<slug>/` | What it does, Gotchas, Related files |
| `gotcha` | `concepts/` | The constraint, The symptom, The fix |

Exactly one `project` page per project. `gotcha` pages sit outside `projects/` because
a constraint learned once applies everywhere — that cross-project reuse is the whole
argument for one central wiki rather than one per repository. They are called
**constraints** in the UI.

`AGENTS.md` is authoritative on all of this and `lint.rs` enforces it. They are one
contract in two languages.

### The primer

Assembled in priority order — project page, decisions, open questions, attempts,
constraints, components — every page whole. A token budget drops from the bottom if it
ever bites; at 300 words a page it rarely does. Flagged pages are named in a
`## Stale, treat with care` section so the model reading the brief knows what to
distrust.

---

## Decision log

Read this before proposing a change to any of it. Each line is here so the same option
is not re-proposed in six months.

| Date | Decision |
|---|---|
| 2026-08-22 | The closing command is how knowledge gets in. The agent that did the work writes the pages; Baton does not parse transcripts. |
| 2026-08-22 | Markdown files are the source of truth. SQLite is a rebuildable index. |
| 2026-08-22 | One central wiki at `~/Baton/`, not one per repository. |
| 2026-08-22 | Baton makes no model calls. No key, no proxy, no cost per use. |
| 2026-08-24 | Every page carries a `#` title stating its claim, and the UI shows that rather than the path. The schema never asked for one, so pages arrived without it and the launcher, `index.md` and every brief fell back to the file path — which is how "draggable-list-full-mobile-rewrite" ended up as a page name. Lint flags a missing title; `[[id\|Title]]` keeps the path as the link target. |
| 2026-08-24 | A deleted wiki folder is an empty one, not an error. `sync` clears the index and every reindex path recreates the folder. Left as an error, `walk` returned before `remove_missing` ran, so rows for vanished files survived forever and the launcher kept offering projects whose files were gone. |
| 2026-08-24 | `/baton` writes silently. Typing it is the approval. **Supersedes** the 2026-08-22 rule that nothing is written without confirmation: a proposal-and-accept prompt at the end of every session is exactly the friction that kills the habit the whole design rests on, and the pages are markdown in a folder the user owns, so a wrong one is corrected by editing it. |
| 2026-08-22 | Staleness is caught by free local checks. Contradiction detection needs a model and is deferred. |
| 2026-08-22 | Humans may hand-edit any page. The agent preserves sections it does not understand. |
| 2026-08-22 | Granularity: a page needs a title someone would plausibly search for. Smaller is a section. Over 300 words means the test was skipped. |
| 2026-08-22 | Section headings come from a fixed allowed set. Ad-hoc headings are how pages stop resembling each other. |
| 2026-08-22 | An `attempt` is never `current`. `superseded` when something replaced it, `abandoned` when it was dropped. |
| 2026-08-22 | Never record an action as done that you did not do. Date every counted number. |
| 2026-08-22 | Positioning: "Raycast, but for context". The differentiators are the corpus, the author and the substrate — not cross-platform. |
| 2026-08-23 | The app creates the wiki silently; installing the skill is always an explicit action, because it writes into another tool's config. |
| 2026-08-23 | The schema and the skill ship inside the binary, from `skills/`, which is their only home. |
| 2026-08-23 | The launcher deals in projects, never pages. The per-type file split is how the wiki organises itself, not a choice to put in front of someone mid-paste. |
| 2026-08-23 | Copying a project copies every page whole. A digest drops a decision's `## Rejected`, which is the only reason that page exists. |
| 2026-08-23 | Launcher search matches project names and page titles, never body text. |
| 2026-08-23 | A `gotcha` is called a **constraint** in the UI. The folder and type names stay: renaming them would break every page id for a label. |
| 2026-08-23 | `/baton` closes with one sentence, not a summary of a session the user just lived through. |
| 2026-08-23 | No schema migrations. One `SCHEMA` constant, fingerprinted; a mismatch drops the index and rebuilds it from the files. Migrations carry data across a change, and there is no data here to carry. |
| 2026-08-23 | Phases 5 and 6 dropped for the MVP. Both optimise delivery; neither addresses whether the pages are worth reading. |

### Rejected, and why

- **Hosted proxy holding our key.** Every conversation would transit our
  infrastructure and we would pay every bill.
- **User supplies their own API key.** Correct while Baton still made model calls;
  the closing command removed the need for any call.
- **JSON in SQLite as the source of truth.** Right for one flat document, wrong for
  many small typed pages written by an agent with file tools.
- **A code index.** Structure is a solved and crowded problem, and a code graph buys
  token efficiency that matters to an agent and barely to a person pressing copy.

---

## Open questions

Resolve each before the phase it blocks, not before starting.

| Question | Blocks | Note |
|---|---|---|
| Does the phase plan belong in the wiki? | — | Wiki pages cite phase numbers a wiki-only reader cannot resolve. Page: `projects/baton/open/plan-lives-outside-the-wiki`. |
| Does `~/Baton/` get a git remote, and is it ever pushed? | — | Free history against a folder that could be pushed by accident. Also the multi-machine answer. |
| Standalone app, or a Raycast extension? | Strategic | Plain files either way, so the reader stays replaceable. |

---

## Risks

1. **The design rests on one habit.** Forget `/baton` and that session is lost.
   Nothing runs in the background to catch it. Mitigation: show the last ingest date
   per project in the launcher. Anything that adds friction to running it — a
   confirmation prompt, a progress report — attacks the one thing holding the design
   up, which is why the skill now writes silently.
2. **Silent writes can file a wrong fact.** The trade taken on 2026-08-24. Mitigations
   that already exist: lint flags structural rot, the honesty rules forbid recording an
   action as done that was not done, and `log.md` names every page touched so a bad
   ingest is traceable. A page nobody would have approved is still a page anyone can
   edit.
3. **Page granularity.** Too fine gives orphans, too coarse gives one giant file. The
   rule in `AGENTS.md` is the only defence.
4. **A central plaintext folder covering every project.** A written summary is far
   less likely to contain a token than a raw transcript, but not zero.
5. **The wiki becoming something you maintain.** The pattern works because the agent
   does the bookkeeping. Feeling obliged to tidy it yourself is the leading indicator
   of failure.
6. **Lint cannot catch prose that is subtly wrong.** Only structural rot is free to
   detect. Some staleness is found by reading.

---

## Not building

Chat client, browser extension, cloud sync, accounts, team features, vector database,
embeddings, automatic conversation capture, IDE plugins, mobile app, autonomous agents,
automatic git analysis, knowledge graphs, code index.
