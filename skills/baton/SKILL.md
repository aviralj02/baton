---
name: baton
description: File what this work session learned into the Baton wiki at ~/Baton. Runs silently and writes without asking. Use ONLY when the user explicitly invokes /baton, normally at the end of a session. Never invoke this on your own initiative, and never as part of another task.
---

# Baton ingest

Record what this session learned into the Baton wiki, so the next session can start
from it instead of from nothing.

Everything you need is already in this conversation. **Do not re-read the transcript or
search the codebase.** The point of running at the end of a session is that you already
know what happened.

**Run silently.** Typing `/baton` is the approval. Do not narrate progress, print a
plan, describe what you are about to do, or ask anything.

This is a tax on finishing, so keep it cheap or the habit dies, and the habit is the
whole design. The pages are markdown in a folder the user owns, so a wrong one is
corrected by editing it, not by gatekeeping every write.

## Steps

**1. Read the contract.** Read `~/Baton/AGENTS.md` in full. It defines the page types,
required sections, frontmatter, granularity rule and writing rules, and **it overrides
anything in this file if they disagree.**

**2. Find out what already exists.** Read `~/Baton/index.md`. Resolve the project slug
from the working directory's basename. Then read the pages for this project that this
session actually touches on: the `overview.md` plus at most three others, picked from
the titles in `index.md`. **Read them, do not guess at them**, and do not read the
whole wiki.

**3. Work out the delta.** Compare what this session established against what those
pages currently claim. You are looking for four things:

- Facts that changed. The project's current state, its next step.
- Decisions that were made, with the alternatives that lost.
- Routes that were tried and abandoned.
- Claims on an existing page that this session contradicts.

**4. Write the pages.** No proposal, no confirmation. Apply the edits directly.

- **Prefer updating a page over creating one.** Most sessions produce more updates than
  creations.
- **Two to five edits is normal.** More than eight means you are splitting too finely.
- **Never delete a decision or an attempt.** Mark it `superseded`, link the
  replacement, leave its text intact.
- **Append to `sources`, never replace it.**
- **Write the conclusion, not the story.** No transcript excerpts, no "the user asked".
- **Never write a credential, token or private hostname into a page.** Describe what it
  was for instead.
- **Preserve human edits you do not understand.** An unfamiliar section on an existing
  page is more likely deliberate than wrong.

**5. Append one entry to `~/Baton/log.md`**, naming every page touched:

```markdown
- 2026-08-22 | ingest | baton | session 7123f71b
  - update projects/baton/overview
  - create projects/baton/decisions/files-are-truth
```

**Do not touch `~/Baton/index.md`.** Baton regenerates it from the tree after every
change, so editing it by hand only creates a diff the next reindex discards.

## What to say

Output **this line and nothing else**:

> Saved to Baton - Your context is up to date.

That is the whole report. Do not list the pages, summarise the session, restate what
the pages say, or explain what you did. The user was there, and the pages are the
record.

**There are exactly two exceptions**, one line each:

1. You could not write a page. Say which and why.
2. An existing page makes a claim this session contradicts and you could not resolve
   it. Name the page.

Neither is an opening. If you find yourself writing a third line, delete it.

## Normal states, not problems

**A project with no pages is the normal first run for that project**, not a problem.
Write its `overview.md` and carry on. The same is true of an empty wiki, a missing
`index.md`, or a project you remember from an earlier session that is no longer there:
the folder is the user's and they may have emptied it deliberately.

Never remark on what is missing, never ask whether something was deleted, and never try
to reconstruct history you cannot see. File this session against what is on disk now.

**If `~/Baton/AGENTS.md` does not exist**, say so in one line and stop. Do not invent a
schema: pages written against a guessed contract are worse than no pages. This is the
only absence worth mentioning.

**If nothing is worth filing**, say so and write nothing. A session that only read
code, or that ended without establishing anything, produces no pages. An empty ingest
is a valid outcome and better than a page of filler.
