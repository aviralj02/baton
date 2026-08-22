---
name: baton
description: File what this work session learned into the Baton wiki at ~/Baton. Use ONLY when the user explicitly invokes /baton, normally at the end of a session. Never invoke this on your own initiative, and never as part of another task.
---

# Baton ingest

Record what this session learned into the Baton wiki, so the next session can start
from it instead of from nothing.

Everything you need is already in this conversation. Do not re-read the transcript,
search the codebase, or gather more context. The point of running at the end of a
session is that you already know what happened.

## Steps

**1. Read the contract.** Read `~/Baton/AGENTS.md` in full. It defines the page types,
required sections, frontmatter, granularity rule and writing rules. It overrides
anything in this file if they disagree.

**2. Find out what already exists.** Read `~/Baton/index.md`. Resolve the project slug
from the working directory's basename. Then read the pages for this project that look
relevant to what this session touched. Read them, do not guess at them.

**3. Work out the delta.** Compare what this session established against what those
pages currently claim. You are looking for four things:

- Facts that changed. The project's current state, its next step.
- Decisions that were made, with the alternatives that lost.
- Routes that were tried and abandoned.
- Claims on an existing page that this session contradicts.

**4. Propose, do not write.** Print a proposal and stop. Format:

```
Project: <slug>   <path>   branch: <branch>

Proposed N edits to ~/Baton/

  update     projects/<slug>/overview.md
             current state, next step
  create     projects/<slug>/decisions/<kebab-title>.md
  supersede  projects/<slug>/decisions/<old>.md
             replaced by <new>
  conflict   <page> says "<claim>"
             this session established "<contradicting claim>"
             will mark that section superseded, not delete it

Accept?  [a] all   [e] one by one   [n] none
```

Order the list `update`, `create`, `supersede`, `conflict`. Keep each line short. For
`create`, do not paste the page body into the proposal.

**5. Write only what is accepted.** On `a`, apply everything. On `e`, ask per edit. On
`n`, stop and write nothing.

After writing:

- Regenerate `~/Baton/index.md` from the tree on disk.
- Append one line to `~/Baton/log.md`:
  `## <ISO date> ingest | <slug> | N edits | session <short id>`
- Report what was written, in one line per file.

## Rules that matter most

- **Prefer updating a page over creating one.** Check `index.md` before proposing a
  `create`. Most sessions should produce more updates than creations.
- **Two to five edits is normal.** More than eight means you are splitting too finely.
  Reconsider before proposing.
- **Never delete a decision or an attempt.** Mark it `superseded`, link the
  replacement, and leave its text intact.
- **Append to `sources`, never replace it.**
- **Write the conclusion, not the story.** No transcript excerpts, no "the user asked",
  no "I then tried".
- **Never write a credential, token or private hostname into a page.** Describe what it
  was for instead.
- **Preserve human edits you do not understand.** An unfamiliar section on an existing
  page is more likely deliberate than wrong.

## If the wiki is not set up

If `~/Baton/AGENTS.md` does not exist, say so and stop. Do not invent a schema.

## If nothing is worth filing

Say so and write nothing. A session that only read code, or that ended without
establishing anything, produces no pages. An empty ingest is a valid outcome and is
better than a page of filler.
