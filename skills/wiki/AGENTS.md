# The Baton wiki schema

This folder is a knowledge base about the projects its owner is building. Agents
write it. Humans read it, and may edit it.

**If you are an agent reading this file, you were told to file what a work session
learned. This document is the contract. Follow it exactly.** Consistency across
sessions and across different agents is the only thing that makes these pages
assemblable later.

---

## Layout

```
~/Baton/
├── AGENTS.md              this file
├── index.md               catalogue of every page, one line each
├── log.md                 append-only record of what happened when
├── projects/
│   └── <slug>/            one directory per project, slug from the folder name
│       ├── overview.md    exactly one per project
│       ├── decisions/
│       ├── attempts/
│       ├── open/
│       └── *.md           component pages sit at this level
└── concepts/              cross-project knowledge, not tied to any one project
```

Resolve `<slug>` from the working directory's basename, lowercased, spaces and
underscores to hyphens. `~/Desktop/personal/baton` becomes `baton`.

---

## Frontmatter

Every page starts with YAML frontmatter. All five fields are required.

```yaml
---
type: decision            # project | decision | open | attempt | component | gotcha
project: baton            # the slug, or `null` for pages in concepts/
status: current           # current | superseded | abandoned | stale
updated: 2026-08-22       # ISO date, the day this page last changed
sources: [7123f71b]       # session ids that produced these claims
---
```

- `sources` is append-only. When you update a page, add your session id, never
  replace the list. It is how a reader traces a claim back to the conversation
  that produced it.
- `updated` is the date you changed the page, not the date of the work.
- Use `project: null` only for pages in `concepts/`.

---

## Page types

### `project`

One per project, always at `projects/<slug>/overview.md`. The root of everything.

Required sections, in this order:

```markdown
## Goal
## Current state
## Next step
```

`Goal` is stable and rarely changes. `Current state` is a short factual paragraph or
list, never a history. `Next step` is one thing, not a list of possibilities.

### `decision`

A choice that was made. Lives in `projects/<slug>/decisions/`.

```markdown
## Decision
## Why
## Rejected
```

`Rejected` is the section that earns this page type. List the alternatives that were
considered and say in one clause why each lost. Without it a later agent will
re-propose exactly what was already ruled out.

Add `## Supersedes` or `## Reverses` with a wiki-link when this decision replaces an
earlier one.

### `open`

A question that is not yet decided. Lives in `projects/<slug>/open/`.

```markdown
## The question
## Options
## What it blocks
```

Use this type, never a `decision` with "not yet made" in it. An open question and a
made decision are read differently, and typing one as the other hides it from anyone
scanning for what is still undecided.

`Options` lists each candidate with its cost in one clause, the same discipline
`Rejected` uses. `What it blocks` names the work that cannot start until this closes.

When the question is answered, write a `decision` page and replace this page's body
with a pointer to it. Set `status` to `superseded`.

### `attempt`

Something that was tried and did not work. Lives in `projects/<slug>/attempts/`.

```markdown
## What was tried
## Why it failed
## What it cost
```

This is the highest-value page type and the one most likely to be skipped. Write it
whenever a session burned effort on a route that was abandoned. `What it cost` can be
time, money, or a dead end that looked promising. Be specific, because vagueness here
is what causes a repeat.

### `component`

A subsystem, module or file. Lives at `projects/<slug>/<name>.md`.

```markdown
## What it does
## Gotchas
## Related files
```

`Related files` uses repo-relative paths, one per line. These paths are what lets a
tool detect that this page has gone stale, so keep them accurate.

### `gotcha`

A constraint discovered the hard way, that applies beyond one project. Lives in
`concepts/`.

```markdown
## The constraint
## The symptom
## The fix
```

Put a gotcha here rather than in a project when it is a fact about a library, an
operating system, or a tool, rather than a fact about this codebase. A gotcha filed
once should never have to be rediscovered in another project.

---

## Sections you may add

A page carries its required sections plus, at most, sections from this list. Do not
invent a heading. Three agents inventing three names for the same thing is what makes
these pages impossible to assemble later.

| Section | Allowed on | Holds |
|---|---|---|
| `## Supersedes` | `decision` | a wiki-link to the page this replaces |
| `## Reverses` | `decision` | a wiki-link to a page this contradicts outright |
| `## Superseded by` | any | a wiki-link to the page that replaced this one |
| `## Known cost` | `decision` | what this choice gives up, accepted deliberately |
| `## Open` | any | one line naming an unresolved question, and a link to its `open` page |

`## Open` is a pointer, never a copy. If a question needs more than one line, it is an
`open` page and this section links to it. Two copies of one question drift apart.

Anything that does not fit a listed section is either a line inside a required section
or a page of its own. Apply the granularity rule below to decide which.

---

## The granularity rule

**A page needs a title someone would plausibly search for. Anything smaller is a
section on an existing page.**

Apply the test literally. "Files are the source of truth" is a page, because someone
would search "source of truth". "Use rename instead of write for atomicity" is not a
page, it is a line under `Gotchas` on the component page for the file store.

Two consequences to respect:

- **Prefer editing an existing page over creating a new one.** Check `index.md` first.
  A wiki that grows only by accretion becomes hundreds of pages nobody reads.
- **A session normally touches two to five pages.** If you are proposing more than
  eight, you are almost certainly splitting too finely. Reconsider before proposing.

---

## Status and the supersede rule

| Status | Meaning |
|---|---|
| `current` | True as far as anyone knows. The default. |
| `superseded` | Replaced by another page. Still true as history. |
| `abandoned` | Dropped with nothing put in its place. No replacement page exists. |
| `stale` | Probably out of date. Nobody has confirmed either way. |

An `attempt` is never `current`. A route that was tried and failed is closed, so it is
`superseded` when a decision replaced it, and `abandoned` when it was simply dropped.
Leaving an attempt `current` makes a dead route read as a live one.

`stale` means nobody has checked, not that something is known to be wrong. Never use
it to mean undecided. Undecided is an `open` page.

**Never delete a decision or an attempt.** When a decision is reversed:

1. Set the old page's `status` to `superseded`.
2. Add `## Superseded by` with a wiki-link to the replacement.
3. Leave every other word of the old page alone.
4. On the new page, add `## Supersedes` or `## Reverses` pointing back.

The reason is that "why did we stop doing it that way" is the most expensive question
in any project, and deleting the old page destroys the only answer.

---

## Wiki-links

Use `[[path/to/page]]`, without the `.md`, relative to the wiki root.

```markdown
See [[projects/baton/decisions/files-are-truth]] and [[concepts/tauri-nspanel]].
```

Every page should link to at least one other page. A page with no links in and none
out is an orphan, and orphans do not get read.

---

## index.md and log.md

`index.md` is a catalogue, one line per page:

```markdown
- [[projects/baton/overview]] the goal, the current state, the next step
```

Group by project, then by type, using the type name as the sub-heading: `Project`,
`Decisions`, `Open`, `Attempts`, `Components`. `concepts/` is not a project, so it
goes last under its own top-level heading. Regenerate it whenever you add or remove a
page. It is what a reader scans first.

`log.md` is append-only. One entry per operation, never edited, never reordered. Name
every page touched, because "3 edits" cannot be traced back to anything:

```markdown
- 2026-08-22 | ingest | baton | session 7123f71b
  - update projects/baton/overview
  - create projects/baton/decisions/files-are-truth
  - supersede projects/baton/decisions/json-not-markdown
```

Use list items, not headings. A heading per entry renders as a wall of headings once
the log has fifty of them.

---

## Writing rules

These pages get pasted into other models as context. Write for that reader.

- **State facts, not narrative.** "The proxy was never deployed" beats "we spent a
  while trying to get the proxy working".
- **One idea per line.** Bullets over paragraphs everywhere except `Goal` and `Why`.
- **No hedging.** If something is uncertain, say what is known and mark the page
  `stale`. Do not write "possibly" or "it seems".
- **Keep a page under about 300 words.** Longer means it should have been two pages,
  or it contains narrative that should be cut.
- **No em dashes and no emojis.**
- **Never copy conversational text in.** No transcript excerpts, no "the user asked",
  no "I then tried". Write the conclusion.
- **Never record an action as done that you did not do.** A session that planned a
  step and did not run it writes the step as outstanding. A page that says a file was
  copied, a service deployed or a command run is read as fact and nobody re-checks it.
  This is the single failure mode that makes a wiki worse than no wiki.
- **Date every counted number.** File counts, sizes and timings are a snapshot of one
  machine on one day. Write "157 files as of 2026-08-22", not "157 files".

## What must never go in a page

- API keys, tokens, passwords, connection strings, or anything that looks like a
  credential. If a session contained one, the page describes what it was for and
  never reproduces it.
- Internal URLs and hostnames that are not already public.
- Raw file contents. Reference the path instead.
- Anything a human has not implicitly agreed to record by asking for the ingest.

---

## Human edits

Humans may edit any page by hand. When you update a page you did not write:

- **Preserve anything you do not understand.** An unfamiliar section is more likely a
  deliberate human addition than a mistake.
- Only rewrite a section when this session actually changed the fact it states.
- Never reformat a page you are not otherwise editing.
