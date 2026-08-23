---
name: baton
description: File what this work session learned into the Baton wiki at ~/Baton. Runs silently and writes without asking. Use ONLY when the user explicitly invokes /baton, normally at the end of a session. Never invoke this on your own initiative, and never as part of another task.
---

# Baton ingest

Record what this session learned into the Baton wiki, so the next session can start
from it instead of from nothing.

Everything you need is already in this conversation. Do not re-read the transcript,
search the codebase, or gather more context. The point of running at the end of a
session is that you already know what happened.

**Run silently.** Typing `/baton` is the approval. Do not narrate progress, do not
print a plan, do not describe what you are about to do, and do not ask anything. Read
what you need, write the pages, say one sentence.

This is a tax on finishing, so keep it cheap or the habit dies — and the habit is the
whole design. A prompt at the end of every session is what kills it: the user has
finished their work and does not want a form to fill in. The pages are markdown in a
folder they own, so a wrong one is corrected by editing it, not by gatekeeping every
write.

## Steps

**1. Read the contract.** Read `~/Baton/AGENTS.md` in full. It defines the page types,
required sections, frontmatter, granularity rule and writing rules. It overrides
anything in this file if they disagree.

**2. Find out what already exists.** Read `~/Baton/index.md`. Resolve the project slug
from the working directory's basename. Then read the pages for this project that look
relevant to what this session touched. Read them, do not guess at them.

**A project with no pages is the normal first run for that project**, not a problem.
Write its `overview.md` and carry on. The same is true of an empty wiki, a missing
`index.md`, or a project you remember from an earlier session that is no longer there:
the folder is the user's and they may have emptied it deliberately. Never remark on
what is missing, never ask whether something was deleted, and never try to reconstruct
history you cannot see. File this session against what is on disk now.

**3. Work out the delta.** Compare what this session established against what those
pages currently claim. You are looking for four things:

- Facts that changed. The project's current state, its next step.
- Decisions that were made, with the alternatives that lost.
- Routes that were tried and abandoned.
- Claims on an existing page that this session contradicts.

**4. Write the pages.** No proposal, no confirmation, no summary of what you are
about to do. Apply the edits directly.

After writing:

- Append one line to `~/Baton/log.md`:
  `## <ISO date> ingest | <slug> | N edits | session <short id>`
- Do **not** touch `~/Baton/index.md`. Baton regenerates it from the tree after every
  change. Editing it by hand only creates a diff that the next reindex discards.

Then output **this line and nothing else**:

> Saved to Baton - Your context is up to date.

That line is the entire output of this command. Not a summary of it. Whatever else
you noticed, however useful it seems, it does not go here.

Specifically, do not:

- list the pages, name them, or count them beyond that line
- summarise the session or restate what the pages say
- explain what you did, what you changed, or why
- report on the state of the wiki: what was missing, what was empty, what looked
  deleted, what you expected to find and did not
- apologise, caveat, flag, observe, or offer to do anything next

**There are exactly two exceptions**, one line each, and nothing else qualifies:

1. You could not write a page. Say which and why, in one line.
2. An existing page makes a claim this session contradicts and you could not resolve
   it. Name the page in one line.

Neither is an opening. If you find yourself writing a third line, delete it. If you
find yourself writing a paragraph, delete it. The user asked a command to file a
session, not for a report on filing it.

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

If `~/Baton/AGENTS.md` does not exist, say so in one line and stop. Do not invent a
schema — pages written against a guessed contract are worse than no pages.

This is the only missing thing worth mentioning. A missing project, a missing
`index.md` or an empty `concepts/` are all normal states; create what you need and say
nothing about it.

## If nothing is worth filing

Say so and write nothing. A session that only read code, or that ended without
establishing anything, produces no pages. An empty ingest is a valid outcome and is
better than a page of filler.
