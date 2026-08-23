//! Composing several pages into one brief for a model.
//!
//! Pure text composition, no model call and no database. Assembling the whole
//! wiki costs well under a millisecond, so the launcher can rebuild this on
//! every summon and show the token estimate before anything is copied.
//!
//! The order below is the point of the file. A reader that runs out of budget
//! must still have the goal, the next step and the decisions already taken,
//! because those are what stop a fresh session re-litigating settled work.

use chrono::NaiveDate;
use serde::Serialize;

use crate::wiki::{Page, PageType, Status};

/// Placeholder in the header, replaced once `fill` knows how many pages
/// actually survived the budget and the live filters.
const PAGE_COUNT: &str = "{{PAGE_COUNT}}";

/// About four characters per token for English prose. Close enough to keep a
/// budget honest, and it costs nothing next to a real tokenizer.
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Primer {
    pub project: String,
    pub text: String,
    /// Estimated tokens, not measured. Shown so a budget can be judged.
    pub tokens: usize,
    pub pages_included: usize,
    /// Pages left out because the budget ran out. Named in the text as well,
    /// because a brief that silently truncates reads as if it were complete.
    pub pages_dropped: usize,
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(CHARS_PER_TOKEN)
}

/// The project a primer should default to: the one whose page changed last.
///
/// The plan wants this read from the frontmost editor workspace. That is worth
/// building when a second project exists and there is something to disambiguate.
pub fn most_recent_project(pages: &[Page]) -> Option<String> {
    pages
        .iter()
        .filter_map(|page| {
            page.frontmatter
                .project
                .as_ref()
                .map(|project| (page.frontmatter.updated, project))
        })
        .max_by_key(|(updated, _)| *updated)
        .map(|(_, project)| project.clone())
}

/// One candidate chunk of the brief, in priority order.
struct Block {
    text: String,
    /// Ids of the pages this block carries. Tracked explicitly rather than
    /// inferred from the text: the project page is rendered as its sections and
    /// its id never appears, so substring matching silently never flags it.
    ids: Vec<String>,
    /// Included whether or not the budget allows. The goal and the next step
    /// are the two things a primer exists to carry.
    required: bool,
}

pub fn assemble(
    pages: &[Page],
    project: &str,
    budget_tokens: usize,
    today: NaiveDate,
    lint: &crate::lint::Report,
) -> Primer {
    let mine: Vec<&Page> = pages
        .iter()
        .filter(|p| p.frontmatter.project.as_deref() == Some(project))
        .collect();

    // Concepts belong to no project on purpose. A constraint learned once
    // applies everywhere, which is the whole argument for one central wiki.
    let concepts: Vec<&Page> = pages
        .iter()
        .filter(|p| p.frontmatter.project.is_none() && p.frontmatter.page_type == PageType::Gotcha)
        .collect();

    let overview = mine.iter().find(|p| p.frontmatter.page_type == PageType::Project);

    // The count is filled in by `fill`, once it knows what actually survived
    // the budget. Counting every page on disk here overstated the brief: live
    // filtering drops superseded decisions, so "from 14 wiki pages" could sit
    // above a document containing nine.
    let mut blocks = vec![Block {
        text: header(project, today),
        ids: Vec::new(),
        required: true,
    }];

    if let Some(page) = overview {
        blocks.push(Block {
            text: format!("{}\n", page.body),
            ids: vec![page.id.clone()],
            required: true,
        });
    }

    // Every page in full, not a summary of it.
    //
    // Copying a project means copying what is known about it. A one-line
    // digest of a decision loses the `## Rejected` section, which is the part
    // that stops the option being re-proposed — so the digest drops exactly the
    // content the page existed to carry.
    //
    // The order is the priority order: if the budget ever runs out, what
    // survives is what a fresh session most needs. It rarely bites — a project
    // is capped at 300 words per page by the schema, so twenty pages fit inside
    // a 12k budget with room to spare.
    for (heading, note, group) in [
        (
            "Decisions already taken",
            "Do not re-propose what a Rejected section already ruled out.",
            live(&mine, PageType::Decision),
        ),
        (
            "Open questions",
            "Undecided. Do not assume an answer.",
            live(&mine, PageType::Open),
        ),
        (
            "Routes already tried",
            "These failed or were dropped. Do not retry one without a new reason.",
            of_type(&mine, PageType::Attempt),
        ),
        (
            "Constraints learned the hard way",
            "Each one cost a debugging session already.",
            concepts.clone(),
        ),
        (
            "Components",
            "Read the constraints on each before touching those files.",
            live(&mine, PageType::Component),
        ),
    ] {
        blocks.extend(full_pages(heading, note, &group));
    }

    fill(blocks, project, budget_tokens, lint)
}

/// Take blocks in order until the budget runs out. A required block goes in
/// whatever the budget says, and once one block is dropped the rest follow, so
/// the brief never ends up with a low-priority page and no high-priority one.
fn fill(
    blocks: Vec<Block>,
    project: &str,
    budget_tokens: usize,
    lint: &crate::lint::Report,
) -> Primer {
    let mut text = String::new();
    let mut included = 0;
    let mut dropped = 0;
    let mut full = false;
    // Which pages actually made it into the brief, so warnings name only pages
    // the reader can see.
    let mut shown: std::collections::HashSet<String> = std::collections::HashSet::new();

    for block in &blocks {
        if block.text.trim().is_empty() {
            continue;
        }
        let fits = estimate_tokens(&text) + estimate_tokens(&block.text) <= budget_tokens;
        if block.required || (!full && fits) {
            text.push_str(&block.text);
            text.push('\n');
            included += block.ids.len();
            shown.extend(block.ids.iter().cloned());
        } else {
            full = true;
            dropped += block.ids.len();
        }
    }

    if dropped > 0 {
        text.push_str(&format!(
            "\n({dropped} more page(s) left out to stay inside the token budget.)\n"
        ));
    }

    // Warnings go in the brief itself, not in a report nobody opens: the model
    // reading this needs to know which lines to distrust. Only pages actually
    // included are named — a warning about a page the reader cannot see is
    // noise.
    let mut flagged: Vec<(&String, &Vec<crate::lint::Finding>)> = lint
        .iter()
        .filter(|(id, _)| shown.contains(*id))
        .collect();
    flagged.sort_by_key(|(id, _)| id.as_str());

    if !flagged.is_empty() {
        text.push_str("\n## Stale, treat with care\n\n");
        text.push_str(
            "These pages are included above but failed a structural check. Prefer the \n\
             repository over anything here that they contradict.\n\n",
        );
        for (id, findings) in flagged {
            for f in findings {
                text.push_str(&format!("- {id} {}\n", f.message()));
            }
        }
    }

    // Substituted now that the real figure is known.
    let text = text.replace(PAGE_COUNT, &included.to_string());

    Primer {
        project: project.to_string(),
        tokens: estimate_tokens(&text),
        text,
        pages_included: included,
        pages_dropped: dropped,
    }
}

fn header(project: &str, today: NaiveDate) -> String {
    format!(
        "# {project}: project context\n\n\
         Assembled by Baton on {today} from {PAGE_COUNT} wiki pages.\n\n\
         Every entry comes from a page written by the agent that did the work. A page\n\
         marked superseded or abandoned is kept deliberately: it is history, and the\n\
         reason something is no longer done that way. Each line carries the date its\n\
         page last changed, so a stale claim can be spotted rather than trusted.\n"
    )
}

/// One block per page, carrying the whole page, under a shared heading.
///
/// Separate blocks rather than one big one so the budget can drop the tail of a
/// section instead of the entire section — losing the last two decisions is far
/// better than losing all of them.
fn full_pages(heading: &str, note: &str, pages: &[&Page]) -> Vec<Block> {
    if pages.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(pages.len());
    for (i, page) in pages.iter().enumerate() {
        // The heading rides on the first page of the group, so a group that is
        // dropped entirely takes its heading with it rather than leaving an
        // empty section in the brief.
        let lead = if i == 0 {
            format!("## {heading}\n\n{note}\n\n")
        } else {
            String::new()
        };
        let status = match page.frontmatter.status {
            Status::Current => String::new(),
            other => format!(" [{}]", other.as_str()),
        };
        let title = if page.title.is_empty() { &page.id } else { &page.title };
        out.push(Block {
            text: format!(
                "{lead}### {title}{status}\n_updated {} · {}_\n\n{}\n",
                page.frontmatter.updated, page.id, body_without_title(page)
            ),
            ids: vec![page.id.clone()],
            required: false,
        });
    }
    out
}

/// A page body with its own `# Title` line removed — the brief re-renders the
/// title as `###` so every page sits at the same depth under its group.
fn body_without_title(page: &Page) -> String {
    page.body
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .skip_while(|l| l.starts_with("# "))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn of_type<'a>(pages: &[&'a Page], want: PageType) -> Vec<&'a Page> {
    pages
        .iter()
        .filter(|p| p.frontmatter.page_type == want)
        .copied()
        .collect()
}

/// Pages of a type that are still true. A superseded decision belongs in the
/// history a reader can look up, not in the list of what currently holds.
fn live<'a>(pages: &[&'a Page], want: PageType) -> Vec<&'a Page> {
    of_type(pages, want)
        .into_iter()
        .filter(|p| p.frontmatter.status == Status::Current)
        .collect()
}

/// The opening of a section, cut at a sentence boundary.
///
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, 23).unwrap()
    }

    fn page(id: &str, kind: &str, project: &str, status: &str, body: &str) -> Page {
        let project = if project.is_empty() { "null" } else { project };
        let text = format!(
            "---\ntype: {kind}\nproject: {project}\nstatus: {status}\n\
             updated: 2026-08-22\nsources: [7123f71b]\n---\n\n{body}"
        );
        crate::wiki::parse(Path::new("/w"), &PathBuf::from(format!("/w/{id}.md")), &text).unwrap()
    }

    fn corpus() -> Vec<Page> {
        vec![
            page(
                "projects/baton/overview",
                "project",
                "baton",
                "current",
                "# Baton\n\n## Goal\n\nA launcher for context.\n\n\
                 ## Current state\n\nThe index works.\n\n\
                 ## Next step\n\nPoint the browser at pages. See [[concepts/mutex-across-await]].\n",
            ),
            page(
                "projects/baton/decisions/files-are-truth",
                "decision",
                "baton",
                "current",
                "# Markdown files are the source of truth\n\n\
                 ## Decision\n\nThe wiki is plain markdown in ~/Baton.\n\n## Why\n\nGit.\n\n## Rejected\n\nSQLite as truth.\n",
            ),
            page(
                "projects/baton/decisions/json-not-markdown",
                "decision",
                "baton",
                "superseded",
                "# The content column holds JSON\n\n## Decision\n\nStore JSON.\n\n## Why\n\nStructure.\n\n## Rejected\n\nMarkdown.\n",
            ),
            page(
                "projects/baton/open/browser-conversations",
                "open",
                "baton",
                "current",
                "# Browser conversations\n\n## The question\n\nHow does a browser chat reach the wiki?\n\n\
                 ## Options\n\nPaste it.\n\n## What it blocks\n\nPhase 4.\n",
            ),
            page(
                "projects/baton/attempts/hosted-proxy",
                "attempt",
                "baton",
                "superseded",
                "# Hosted proxy\n\n## What was tried\n\nA Worker holding the key.\n\n\
                 ## Why it failed\n\nEvery conversation would transit our infrastructure.\n\n## What it cost\n\nA day.\n",
            ),
            page(
                "concepts/mutex-across-await",
                "gotcha",
                "",
                "current",
                "# Never hold a std Mutex across an await\n\n\
                 ## The constraint\n\n**A guard held across an `.await`** keeps the lock for the whole future.\n\n\
                 ## The symptom\n\nThe UI stalls.\n\n## The fix\n\nDrop the guard first.\n",
            ),
        ]
    }

    #[test]
    fn the_brief_leads_with_the_project_page_then_the_decisions() {
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());

        let goal = p.text.find("## Goal").expect("the goal must be carried");
        let decisions = p.text.find("## Decisions already taken").unwrap();
        let open = p.text.find("## Open questions").unwrap();
        let attempts = p.text.find("## Routes already tried").unwrap();
        assert!(goal < decisions && decisions < open && open < attempts);

        assert!(p.text.contains("Assembled by Baton on 2026-08-23"));
        assert!(p.text.contains("Point the browser at pages"), "next step is missing");
    }

    #[test]
    fn a_superseded_decision_is_not_offered_as_current() {
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        assert!(p.text.contains("Markdown files are the source of truth"));
        assert!(
            !p.text.contains("- The content column holds JSON"),
            "a reversed decision must not read as one that still holds"
        );
    }

    #[test]
    fn an_abandoned_route_is_carried_with_the_reason_it_failed() {
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        assert!(p.text.contains("Hosted proxy [superseded]"));
        assert!(p.text.contains("Every conversation would transit our infrastructure"));
    }

    #[test]
    fn the_budget_drops_from_the_bottom_and_says_so() {
        let full = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        let tight = assemble(&corpus(), "baton", 200, today(), &Default::default());

        assert!(tight.tokens < full.tokens);
        assert!(tight.pages_dropped > 0);
        assert!(tight.text.contains("left out to stay inside the token budget"));
        // Whatever else goes, the goal and the next step stay.
        assert!(tight.text.contains("## Goal"));
        assert!(tight.text.contains("Point the browser at pages"));
    }

    #[test]
    fn an_empty_section_is_left_out_rather_than_left_blank() {
        let only_overview = vec![corpus().remove(0)];
        let p = assemble(&only_overview, "baton", 10_000, today(), &Default::default());
        assert!(p.text.contains("## Goal"));
        assert!(!p.text.contains("## Open questions"));
        assert!(!p.text.contains("## Routes already tried"));
    }

    #[test]
    fn concepts_are_included_even_though_they_belong_to_no_project() {
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        // A constraint learned once applies everywhere, which is the whole
        // argument for one central wiki rather than one per repository.
        assert!(p.text.contains("Never hold a std Mutex"));
        // And it arrives whole, not as a digest: the fix is the point.
        assert!(p.text.contains("A guard held across an `.await`"));
    }

    #[test]
    fn a_decision_arrives_with_its_rejected_section_intact() {
        // Why whole pages: a one-line digest drops `## Rejected`, the section
        // that stops the option being re-proposed.
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        assert!(p.text.contains("## Rejected"));
        assert!(p.text.contains("SQLite as truth"), "rejected alternative dropped");
    }

    #[test]
    fn a_brief_carrying_a_flagged_page_says_so_in_its_own_text() {
        let pages = corpus();
        let flagged = pages.iter()
            .find(|p| p.frontmatter.page_type == PageType::Project)
            .expect("corpus has an overview").id.clone();
        let mut lint = crate::lint::Report::new();
        lint.insert(flagged.clone(), vec![crate::lint::Finding::TooLong { words: 412 }]);

        let p = assemble(&pages, "baton", 10_000, today(), &lint);
        assert!(p.text.contains("## Stale, treat with care"));
        assert!(p.text.contains(&flagged));
        assert!(p.text.contains("412 words"));
    }

    #[test]
    fn a_clean_wiki_gets_no_warning_section() {
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        assert!(!p.text.contains("Stale, treat with care"));
    }

    #[test]
    fn a_warning_about_a_page_that_was_dropped_is_not_shown() {
        // Warning about a page the reader cannot see is noise.
        let mut lint = crate::lint::Report::new();
        lint.insert("projects/baton/not-in-this-brief".to_string(),
                    vec![crate::lint::Finding::Orphan]);
        let p = assemble(&corpus(), "baton", 10_000, today(), &lint);
        assert!(!p.text.contains("not-in-this-brief"));
    }

    #[test]
    fn the_header_counts_what_the_brief_actually_carries() {
        // Counting every page on disk overstated it: live filtering drops
        // superseded decisions, so the header claimed more than was included.
        let p = assemble(&corpus(), "baton", 10_000, today(), &Default::default());
        assert!(
            p.text.contains(&format!("from {} wiki pages", p.pages_included)),
            "header disagrees with pages_included: {}",
            p.text.lines().take(3).collect::<Vec<_>>().join(" ")
        );
        assert!(!p.text.contains("PAGE_COUNT"), "placeholder leaked into the brief");
    }

    #[test]
    fn the_default_project_is_the_one_touched_last() {
        assert_eq!(most_recent_project(&corpus()).as_deref(), Some("baton"));
        assert_eq!(most_recent_project(&[]), None);
    }
}
