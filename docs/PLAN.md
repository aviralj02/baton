# Baton build plan

**This file is the living record of what is done and what is next.** Update it in
place as work lands. Do not start a parallel status doc.

**This is the only design document.** `baton-the-wiki.html` was deleted on 2026-08-23.
Its vision and worked example are folded in below, so nothing needs reading alongside
this file.

- **Product spec and history:** `PRD.md`. It still describes the pre-wiki design in
  places. Phase 4 reconciles it.
- **Working rules and gotchas:** `CLAUDE.md`.
- **The knowledge itself:** `~/Baton/`. `index.md` lists every page, `log.md` records
  every ingest, `AGENTS.md` is the schema. Trust those over any list in this file.

Status values: `todo`, `doing`, `done`, `blocked`, `dropped`.

---

## Where we are

**The loop runs.** Schema, command, wiki, page reading, search, primer and launcher are
all built. Phases 0 through 3 all landed inside two days, and the next real work is the
prune in Phase 4.

| Phase | What it delivers | Status |
|---|---|---|
| 0 | Validate the idea with no code | `done`, accepted untested |
| 1 | The wiki store in Rust | `doing`, watcher outstanding |
| 2 | The primer, and the launcher reading it | `done` |
| 3 | Lint checks, surfaced inline | `doing`, `broken_links` shipped |
| 4 | Prune what the new design makes dead | `todo`, next |
| 5 | MCP server | `todo` |
| 6 | Optional backfill from old transcripts | `todo` |

Phases 1 and 2 ran ahead of this file and were reconciled against the code on
2026-08-23. Several Phase 1 tasks turned out to be `dropped` by design rather than
pending. Read each phase for detail, and trust the code over this table.

The wiki holds 19 pages as of 2026-08-23: 1 project, 5 decisions, 3 open questions, 2
attempts, 3 components, 5 gotchas.

### Phase 0 was accepted without its acceptance test

Decided 2026-08-23. The cold test, pasting a brief into a fresh window to see whether
it needs correcting, was skipped and the loop assumed to work.

That is a defensible bet. The pages read well, and the primer's ordering is pinned by
ten tests. But it is the one assumption nothing else covers, and the whole design rests
on it. **If handoffs start feeling thin, look here first.** The fix would almost
certainly be a line in `~/Baton/AGENTS.md` rather than anything in the Rust.
`~/Baton/.acceptance-test.md` is still on disk if it is ever worth running.

### Verified state of the repo, 2026-08-23

- `cargo test --lib` passes, **53 of 53**, up from 17. New `wiki` and `primer` suites.
- `pnpm build` passes, no type errors. Bundle 270 kB.
- Rust modules now include `wiki.rs` and `primer.rs`. `ai.rs` is still present and still
  registered, awaiting Phase 4.
- The launcher searches wiki pages and offers the assembled brief as its first row. The
  browser lists and opens pages.
- Both the page commands and the nine older `contexts` commands are live side by side.
  Phase 4 removes the older set.
- AI generation has still never run, because the proxy was never deployed. Phase 4
  removes the need for it rather than fixing it.

---

## The loop, in practice

Carried over from the deleted vision doc, because it is the clearest statement of what
the product is.

**1. At the end of a session**, in any tool with the skill installed:

```
> /baton

Project: baton   ~/Desktop/personal/baton   branch: main

Proposed 3 edits to ~/Baton/

  update     projects/baton/overview.md
  create     projects/baton/decisions/files-are-truth.md
  conflict   architecture.md says "content holds JSON"
             this session decided files are the source of truth

Accept?  [a] all   [e] one by one   [n] none
```

The `conflict` line is the one that matters. It is the wiki saying it already believed
something else, which is the moment a knowledge base earns its keep.

**2. Next session**, the hotkey offers the brief as its first row, already filtered to
the project touched last. One keystroke copies something like this:

```markdown
# Baton, continuation context

## Goal
A local-first launcher that hands your context to any AI tool.

## Next step
Retire ai.rs and the nine contexts commands.

## Decisions standing
- Markdown files are the source of truth, SQLite is an index.
- The closing command is how knowledge gets in.

## Already tried, do not repeat
- Embedding an API key in the binary. Recoverable with `strings`.

## Stale, treat with care
- architecture.md still describes JSON. Superseded 2026-08-22.
```

Not one page. Assembled from several, in priority order, with staleness carried inline
so the next model knows what to distrust.

**Positioning:** Raycast puts a command a keystroke away, this puts what you know a
keystroke away. The differentiators are the corpus, the author and the substrate, never
cross-platform, since Raycast has been on Windows since late 2025. Detail lives in
`~/Baton/projects/baton/decisions/positioning-raycast-for-context`.

---

## The shape, in one paragraph

A central folder of markdown files at `~/Baton/` holds a wiki about every project.
At the end of an agent session the user runs `/baton`, and the agent that did the
work proposes page edits, which the user approves. Baton itself never calls a model.
It indexes the files, and one hotkey assembles a primer from several pages onto the
clipboard. Three parts: a skill file the agent runs, a folder of markdown, and a
launcher that reads it.

---

## Phase 0: Validate with no code

**Goal:** find out whether an agent asked to file what it learned writes pages worth
reading. This is the riskiest assumption in the whole design and it costs about an
hour to test.

**Write no Rust in this phase.**

### Tasks

- [x] Create `~/Baton/` with `projects/` and `concepts/` subdirectories.
- [x] Write `~/Baton/AGENTS.md`, the schema. It must define:
  - [x] The five page types and their required sections.
  - [x] The frontmatter fields: `type`, `project`, `status`, `updated`, `sources`.
  - [x] The `status` values and what each means.
  - [x] The granularity rule, stated as a test the agent can apply.
  - [x] The rule that a decision is never deleted, only superseded and linked.
  - [x] Wiki-link syntax and how paths resolve.
- [x] Write `~/.claude/skills/baton/SKILL.md`. Roughly forty lines.
- [x] Hand-write `~/Baton/projects/baton/overview.md` so there is something to update.
- [x] Hand-write `~/Baton/index.md` and an empty `~/Baton/log.md`.
- [x] Run `/baton` at the end of a real session in this repo.
- [x] Read every page it wrote. Note every edit you would have made. See
      **Judged 2026-08-22** below.
- [dropped] Run the acceptance test in a genuinely cold session. Skipped by decision on
      2026-08-23. See **Phase 0 was accepted without its acceptance test** above.

### Seeded 2026-08-22

Nine pages, covering every page type, written by hand from this session's own work.
They exist so the acceptance test has something real to update rather than an empty
folder.

```
projects/baton/overview.md                              project
projects/baton/decisions/closing-command-is-the-ingest  decision
projects/baton/decisions/files-are-truth                decision
projects/baton/decisions/baton-makes-no-model-calls     decision
projects/baton/decisions/json-not-markdown              decision, superseded
projects/baton/attempts/hosted-proxy                    attempt
projects/baton/attempts/transcript-mining               attempt
projects/baton/open/browser-conversations               open question
concepts/tauri-nspanel                                  gotcha
```

A schema check over these nine passes clean: no missing frontmatter, no missing
required sections, 11 wiki-links with none broken, no orphans, nothing absent from
`index.md`, nothing over the word limit. That check is a throwaway script for now and
becomes `lint.rs` in Phase 3.

Deferring the `component` type was a mistake, and the judgement below explains why. The
repair added three: `projects/baton/db`, `projects/baton/launcher` and
`projects/baton/wiki`. The last one documents a module that does not exist yet, which
turns out to be useful: it carries the Phase 1 design decisions where a cold reader will
find them.

### Judged 2026-08-22

The page types work. The riskiest assumption held: an agent asked to file what it
learned wrote real decision pages with real `Rejected` sections, a correct supersede
chain and no narrative padding. The ingest still failed, on two counts.

**It filed every decision and skipped almost every constraint.** Of the ten gotchas in
`CLAUDE.md`, four reached the wiki and two more die with `ai.rs` in Phase 4. Four live
ones were missing, along with the `user_version` migration rule, the built-once
launcher rule and the external-content FTS rule. A cold session would have re-hit all
of them. Deferring the `component` type deferred the content that carries them.

**It recorded an action as done that was never done.** The ingest wrote that the skill
file was copied to all three tool directories. Only the Claude Code copy existed. This
is the worst failure mode available to a wiki, because nobody re-checks a stated fact.

Two smaller factual errors: the Codex session count was 21 against a real 18, and
`files-are-truth` claimed git history as a benefit while `~/Baton/` was not a
repository.

Four gaps in the schema itself, all now closed in `AGENTS.md`:

- `open/` appeared in the layout with no matching page type, so the ingest typed an
  open question as a `decision` reading "not yet made" and marked it `stale`.
- Five ad-hoc section headings appeared across four pages. The allowed set is now a
  table, and `## Open` is a one-line pointer rather than a second copy of a question.
- No status value meant "dead", so both attempts read `current` while their bodies
  said superseded. Added `abandoned`, and the rule that an attempt is never `current`.
- `log.md` recorded "3 edits" with no page names, so nothing was traceable.

Also added: never record an action as done that you did not do, and date every counted
number.

**Repair applied the same day.** Schema check passes: every required section present,
all frontmatter complete, no page over 300 words, no broken links, no orphans, nothing
missing from `index.md`. Verified 2026-08-23 that the skill really is copied to all
three tool directories, that `~/Baton/` really is a git repository with no commit yet
made, and that the corrected Codex count of 18 matches the machine. Checking the claims
of a repair whose headline finding was an unchecked claim seemed like the minimum.

**Second ingest 2026-08-23**, session `eb76c6df`, four edits. It added
`projects/baton/wiki` and raised a question the design had not noticed: the phase plan
lives in the app repo where the wiki cannot see it, yet wiki pages cite Phase 1 and
Phase 4 by number. See `projects/baton/open/plan-lives-outside-the-wiki`. That is the
ingest loop doing its job, finding a real gap nobody had written down.

Phase 0 does not pass until the cold acceptance test runs against the repaired pages.

### Page types and required sections

| Type | Lives in | Required sections |
|---|---|---|
| `project` | `projects/<slug>/overview.md` | Goal, Current state, Next step |
| `decision` | `projects/<slug>/decisions/` | Decision, Why, Rejected |
| `open` | `projects/<slug>/open/` | The question, Options, What it blocks |
| `attempt` | `projects/<slug>/attempts/` | What was tried, Why it failed, What it cost |
| `component` | `projects/<slug>/` | What it does, Gotchas, Related files |
| `gotcha` | `concepts/` | The constraint, The symptom, The fix |

There is exactly one `project` page per project. `gotcha` pages live outside
`projects/` on purpose, because a constraint learned once applies everywhere. That
cross-project reuse is the whole argument for one central wiki rather than one per
repository.

### Acceptance test

Paste the pages the agent wrote into a fresh agent window and start working.

- **Pass:** the new session needs no correction from you in its first few exchanges.
- **Fail:** you find yourself re-explaining something the pages should have carried.

On a fail, the fault is almost always `AGENTS.md`, not the model. Fix the schema and
run again. Do not proceed to Phase 1 until this passes, because every later phase
assumes these pages are worth assembling.

### Notes

- The skill directory exists for all three tools on this machine:
  `~/.claude/skills/`, `~/.codex/skills/`, `~/.cursor/skills/`. Write the skill once
  and copy it. Claude Code also supports `~/.claude/commands/`.
- Record the granularity rule you settle on directly in `AGENTS.md`. It is the single
  thing keeping three different agents writing consistent pages.

---

## Phase 1: The wiki store

**Goal:** Baton reads the wiki folder and the launcher searches it. The `contexts`
table stops being the source of truth and becomes a rebuildable index.

**Status `doing`, and the goal narrowed.** The original goal said "reads and writes".
Baton does not write. The agent writes through the skill, which is the whole of
[[projects/baton/decisions/closing-command-is-the-ingest]]. Every write-path task below
is therefore dropped rather than pending, and that is a design conclusion, not a
shortcut.

### Tasks

- [x] Decide and persist the wiki root. Default `~/Baton/`, resolved in
      `lib.rs::wiki_root`. A missing folder degrades the feature instead of failing the
      launch.
- [x] `wiki.rs`: read a page, parse YAML frontmatter, parse the body into sections.
- [x] Parse `[[wiki-links]]` and resolve them. `broken_links` and `page_backlinks`
      commands expose the results.
- [x] Rewrite `db.rs` as an index over pages, FTS5 over title and body.
- [x] Reindex through the `sync_wiki` command, plus a sweep at startup.
- [x] Commands: `list_pages`, `search_pages`, `read_page`, `sync_wiki`.
- [x] Point `Launcher.tsx` and `Browser.tsx` at pages instead of contexts.
- [ ] File watcher over the wiki root. Debounce, because editors write more than once.
      Still needed, since `sync_wiki` is on demand and the acceptance test below wants
      an external edit reflected without one. No `notify` dependency yet.
- [ ] Retire the `contexts` commands. Nine still live in `commands.rs` beside the page
      commands. Moved to Phase 4, since it is deletion rather than construction.
- [dropped] Write a page atomically. Baton has no write path and does not need one.
- [dropped] Regenerate `index.md` from the tree. The skill does this.
- [dropped] Append to `log.md`. The skill does this.
- [dropped] `write_page` and `delete_page` commands. A second write path is a second
      place for the schema to be violated.

### Acceptance

- [ ] Hand-edit a page in an external editor. The launcher reflects it within a second.
      **Blocked on the watcher.**
- [x] Delete the SQLite file. Restart. Everything is back.
- [x] Search finds a page by a word in its body, not just its title.
- [x] `cargo test --lib` green. 53 tests as of 2026-08-23, up from 17, with `wiki` and
      `primer` suites added and the db tests rewritten against pages.

### Gotchas

- The FTS index is external-content and kept in sync by triggers. It is now derived
  from files rather than from rows, so the triggers may not be the right mechanism any
  more. Rebuild-on-change is simpler and correctness matters more than speed here.
- Do not hold the DB mutex across a file read.
- A watcher event can fire for a file the agent is mid-write on. Read only complete
  files, and tolerate a parse failure without poisoning the index.

---

## Phase 2: The primer, and the launcher

**Goal:** one keystroke puts an assembled brief on the clipboard. No model call.

**Status `doing`, essentially complete.** `primer.rs` exists with 10 tests. What is left
is judging the output in real use rather than writing more code.

### Tasks

- [x] `primer.rs`: assemble in priority order, project page first, linked pages last.
- [x] Token budget. `estimate_tokens` at about four characters per token, fill from the
      top, drop from the bottom, and name what was dropped in the text.
- [x] Detect the current project. `most_recent_project` picks the one touched last.
- [x] Launcher offers the brief as the first row.
- [x] `copy_primer` and `copy_page` commands. Copying one page stayed as the second
      action.
- [x] Show the token estimate before copying.

### Acceptance

- [ ] Press the hotkey and paste into a new agent window. It is up to speed. **This is
      the Phase 0 cold test, and it gates both phases.**
- [x] The primer never exceeds its budget, and never drops the goal or the next step.
      Covered by `the_budget_drops_from_the_bottom_and_says_so`.
- [x] With no typing at all, the right project is offered first. Covered by
      `the_default_project_is_the_one_touched_last`.

### What the tests already pin

Worth knowing, because these are the behaviours a later change could silently break: a
superseded decision is not offered as current, an abandoned route is carried with the
reason it failed, `concepts` pages are included despite belonging to no project, a page
the overview links to is carried whole, an empty section is left out rather than left
blank, and a period inside a path or a version number does not end a sentence.

### Notes

Because there is no model call, this can run on every keystroke. A live preview of the
primer as you move the selection is affordable and worth doing.

---

## Phase 3: Lint, surfaced inline

**Goal:** stale pages announce themselves in the primer, not in a report nobody opens.

### Checks, all free and deterministic

- [ ] A page cites a file that no longer exists.
- [ ] A cited file changed after the page's `updated` date.
- [ ] A page's newest source predates N commits on the current branch.
- [x] A `[[wiki-link]]` does not resolve. Shipped as the `broken_links` command, which
      is why Phase 3 reads `doing`. Strip fenced blocks **and inline code spans**
      before scanning. A throwaway checker on 2026-08-23 reported
      `projects/baton/wiki` as having a broken link, because that page quotes the
      literal syntax inside backticks while describing the parser. Same class of bug as
      the example link in `AGENTS.md`, one nesting level down.
- [ ] An orphan page. Nothing links in, and it links nowhere.
- [ ] Two `current` decision pages on the same subject.
- [ ] A page missing from `index.md`.
- [ ] An `attempt` page with `status: current`. An attempt is always closed.
- [ ] A section heading outside the allowed set in `AGENTS.md`.

### Tasks

- [ ] `lint.rs`, returning a list of findings per page.
- [ ] Run on index, cache per page, invalidate on change.
- [ ] Mark flagged pages in the launcher list.
- [ ] Add a "Stale, treat with care" section to the primer when any included page is
      flagged, naming the page and the reason.

### Acceptance

- [ ] Rename a file a page cites. The page is flagged without a restart.
- [ ] A primer including a stale page says so in its own text.

### Deferred

Contradiction detection between two pages needs a model. Offer it later as a separate
manual command, not as part of the automatic pass.

---

## Phase 4: Prune

**Goal:** delete what the new design makes dead, and reconcile the PRD. Do this only
after Phase 2 passes, so nothing is removed on the strength of a plan alone.

### Tasks

- [ ] Retire the nine `contexts` commands in `commands.rs`, now that the page commands
      cover the same ground. Carried over from Phase 1, since it is deletion.
- [ ] Delete `src-tauri/src/ai.rs`, still present and still registered in `lib.rs`.
- [ ] Remove the `create_context_from_conversation`,
      `update_context_from_conversation` and `generate_handoff` commands.
- [ ] Remove `PasteConversation.tsx`, or keep it wired to the browser-chat path if
      that open question resolves in favour of keeping a key.
- [ ] Remove the `keyring` dependency, unless the browser path keeps a key.
- [ ] Keep `proxy/` in the repo, dormant and documented as optional. Do not delete it.
- [ ] Remove `reqwest` if nothing else needs it.
- [ ] Update `PRD.md`:
  - [ ] Record the reversal of decision 2. JSON in SQLite becomes markdown files.
  - [ ] Record that the user supplies no key because Baton makes no model calls.
  - [ ] Mark Milestone 3's two blockers as dropped rather than done.
  - [ ] Rewrite section 9 and section 14. Baton now sends nothing anywhere.
  - [ ] Resolve section 14b. Baton no longer stores raw conversations at all.
- [ ] Update `CLAUDE.md`: remove the Anthropic API section, remove architecture rule 5
      about provider abstraction, add the wiki layout.

### Acceptance

- [ ] `cargo build` green with no unused-dependency warnings.
- [ ] Grep for `api_key`, `x-api-key` and `BATON_API_BASE` returns only `proxy/`.
- [ ] `PRD.md` contains no statement that contradicts the shipped design.

---

## Phase 5: MCP server

**Goal:** agents fetch the primer themselves, so pasting becomes the fallback rather
than the main path.

### Tasks

- [ ] Expose the primer as an MCP tool, taking a project and a token budget.
- [ ] Expose page read and page search as tools.
- [ ] Decide whether writes go through MCP or stay with the skill's file tools.
- [ ] Document installation for each client.

### Acceptance

- [ ] A fresh agent session with no paste can answer what the project's next step is.
- [ ] The hotkey still works unchanged, because browser chats have no MCP.

### Note

Writes probably should not go through MCP. The skill already has file tools, and a
second write path is a second place for the schema to be violated.

---

## Phase 6: Optional backfill

**Goal:** recover the history that predates the skill.

Counted 2026-08-22: 157 Claude Code transcripts totalling 748 MB, 18 Codex sessions,
and 14 Cursor workspaces. None can be captured by `/baton`, because those sessions are
over. Reading them is the only route. The count grows every day this machine is used,
so re-count before sizing the work.

### Tasks

- [ ] Reader for Claude Code. `~/.claude/projects/<slug>/<uuid>.jsonl`. One JSON object
      per line. `parentUuid` makes each session a tree, not a list. `cwd` and
      `gitBranch` are on most records.
- [ ] Reader for Codex. `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Uniform
      `{timestamp, type, payload}` envelope. Types seen: `session_meta`, `event_msg`,
      `response_item`, `turn_context`.
- [ ] Reader for Cursor. SQLite at
      `~/Library/Application Support/Cursor/User/workspaceStorage/<hash>/state.vscdb`.
      Table `ItemTable`, keys `aiService.generations` and `composer.composerData`.
      `workspace.json` gives the folder.
- [ ] Group sessions by project directory.
- [ ] Redact by pattern before anything is shown or sent.
- [ ] Emit a summary per session for a human or an agent to file.

### Decide before building

This is a one-time tool. It may belong as a standalone script rather than inside the
app. It also reintroduces the only reason Baton would need a model, so consider having
it emit prepared prompts for an agent to process instead of calling anything itself.

---

## Carried over from the old plan

Still valid, unrelated to the wiki direction.

- [ ] User-configurable global shortcut, persisted.
- [ ] Measure and defend launcher show latency. Hold well under 100ms.
- [ ] First real Windows test pass.
- [ ] Code signing. Apple Developer account for notarisation, and a Windows
      certificate. Blocks distribution, not development.

---

## Decision log

| Date | Decision | Note |
|---|---|---|
| 2026-08-22 | The closing command is how knowledge gets in | The agent that did the work writes the pages. Baton does not parse transcripts in the normal path. |
| 2026-08-22 | Markdown files are the source of truth | SQLite becomes a rebuildable index. **Reverses PRD decision 2.** |
| 2026-08-22 | One central wiki at `~/Baton/`, not one per repository | Cross-project reuse of `gotcha` pages is the point. |
| 2026-08-22 | The hotkey assembles a primer, not a single page | Copying one page stays as a secondary action. |
| 2026-08-22 | Nothing is written without approval | Applies to the skill and to any future MCP write. |
| 2026-08-22 | Staleness is caught by free local checks | Model-driven contradiction detection deferred and manual. |
| 2026-08-22 | Baton makes no model calls | So no key, no proxy, no rate limit, no cost per use. Supersedes the user-supplied-key decision taken earlier the same day, which itself superseded the hosted proxy. |
| 2026-08-22 | Human reads it first, agents second | Same files, MCP added in Phase 5. |
| 2026-08-22 | Granularity rule: a page needs a title someone would plausibly search for | Anything smaller is a section on an existing page. A session normally touches two to five pages, and more than eight means it is splitting too finely. Lives in `AGENTS.md`. |
| 2026-08-22 | Humans may hand-edit any page | More useful than the strict version where only the agent writes. The agent must preserve sections it does not understand, and only rewrite a section when the session actually changed the fact it states. |
| 2026-08-22 | Positioning is "Raycast, but for context" | Raycast reached Windows in beta in late 2025, so cross-platform is no longer a differentiator against it. The differentiators are the corpus, the author, and the substrate. |
| 2026-08-22 | `open` is a page type, not a `decision` with an empty body | Sections are The question, Options, What it blocks. The first ingest typed an open question as a decision reading "not yet made", because `open/` was in the layout with no matching type. |
| 2026-08-22 | Section headings come from a fixed allowed set | Five ad-hoc headings appeared across four pages in one ingest. `## Open` is now a one-line pointer to an `open` page, never a second copy of the question. |
| 2026-08-22 | `abandoned` joins the status vocabulary, and an `attempt` is never `current` | `superseded` when a decision replaced it, `abandoned` when it was simply dropped. Without this a dead route reads as a live one. |
| 2026-08-22 | `log.md` names every page it touched | The first ingest logged "3 edits" with no names, so nothing was traceable. |
| 2026-08-22 | Never record an action as done that you did not do, and date every counted number | The first ingest claimed the skill was installed for three tools when only one copy existed. This is the worst failure mode a wiki has, because nobody re-checks a stated fact. |
| 2026-08-23 | Components get pages before the code exists | `projects/baton/wiki` documents a module not yet written. Deferring the `component` type deferred the gotchas that carry the real constraints, which was the main coverage failure of the first ingest. |

### Superseded decisions, kept for the record

- **Hosted proxy holding our key.** Worked, but every conversation would transit our
  infrastructure and we would pay every bill. Superseded 2026-08-22.
- **User supplies their own API key, held in the keychain.** Correct while Baton still
  made model calls. Superseded the same day once the closing command removed the need
  for any call.
- **`contexts.content` holds JSON, not markdown.** Correct for one flat seven-field
  document. Wrong for many small typed pages, and wrong now that the writer is an agent
  with file tools. Superseded 2026-08-22.

---

## Open questions

Each one blocks a specific phase. Resolve before that phase, not before starting.

| Question | Blocks | Notes |
|---|---|---|
| ~~What is the granularity rule?~~ | ~~Phase 0~~ | **Resolved 2026-08-22.** See the decision log. Revisit if the wiki starts accumulating pages nobody reads. |
| ~~Do you hand-write pages as well?~~ | ~~Phase 0~~ | **Resolved 2026-08-22.** Yes. See the decision log. |
| What happens to browser conversations? | Phase 4 | Tracked as `projects/baton/open/browser-conversations`. Deciding it decides whether `keyring` and `ai.rs` survive. |
| Does the phase plan belong in the wiki? | Phase 1 | **New, raised by the 2026-08-23 ingest.** Wiki pages cite phase numbers that a wiki-only reader cannot resolve. Tracked as `projects/baton/open/plan-lives-outside-the-wiki`. |
| Standalone app, or a Raycast extension? | Strategic, not blocking | Tracked as `projects/baton/open/standalone-or-raycast-extension`. Plain files either way, so the reader stays replaceable. |
| Does the wiki folder get a remote, and is it ever pushed? | Phase 1 | Partly answered: `git init` has been run, no commit made, no remote. What is left is whether it is ever pushed, which is also the multi-machine answer. |
| Is the backfill worth building? | Phase 6 | 157 files and 748 MB as of 2026-08-22. Possibly a script rather than app code. |

Open questions that have a wiki page are tracked there, not here. This table exists to
map each one to the phase it blocks. When one closes, the wiki page becomes a
`decision` and this row gets struck through.

---

## Risks

1. **The design rests on one habit.** Forget to run `/baton` and that session is lost.
   Nothing runs in the background to catch it. Mitigation: show the last ingest date
   per project in the launcher.
2. **Page granularity.** Too fine gives hundreds of orphans, too coarse gives one
   giant file. The rule in `AGENTS.md` is the only defence.
3. **A central plaintext folder covering every project.** Smaller than the old risk,
   because a written summary is far less likely to contain a token than a raw
   transcript, but not zero.
4. **The wiki becoming something you maintain.** The pattern works because the agent
   does the bookkeeping. Feeling obliged to tidy it yourself is the leading indicator
   of failure.
5. **Lint cannot catch prose that is now subtly wrong.** Only structural rot is free to
   detect. Accept that some staleness is found by reading, and make fixing a page you
   are looking at a single action.

---

## Not building

Unchanged from the PRD, plus the additions this direction rules out.

Chat client, browser extension, cloud sync, accounts, team features, vector database,
embeddings, automatic conversation capture, IDE plugins, mobile app, autonomous agents.

New to this list: a code index. Structure is a crowded and solved problem, and the
measured evidence says a code graph buys token efficiency that matters to an agent and
barely to a person pressing copy. Revisit only after Phase 5, when agents are actually
reading the wiki.
