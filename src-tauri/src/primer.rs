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

/// About four characters per token for English prose. Close enough to keep a
/// budget honest, and it costs nothing next to a real tokenizer.
const CHARS_PER_TOKEN: usize = 4;

/// Cap on a one-line summary, in characters, before it is trimmed at a word.
const SUMMARY_CHARS: usize = 160;

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
    pages: usize,
    /// Included whether or not the budget allows. The goal and the next step
    /// are the two things a primer exists to carry.
    required: bool,
}

pub fn assemble(pages: &[Page], project: &str, budget_tokens: usize, today: NaiveDate) -> Primer {
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

    let mut blocks = vec![Block {
        text: header(project, mine.len() + concepts.len(), today),
        pages: 0,
        required: true,
    }];

    if let Some(page) = overview {
        blocks.push(Block {
            text: format!("{}\n", page.body),
            pages: 1,
            required: true,
        });
    }

    blocks.push(summary_block(
        "Decisions already taken",
        "Do not re-propose what a Rejected line already ruled out.",
        &live(&mine, PageType::Decision),
        "Decision",
    ));

    blocks.push(summary_block(
        "Open questions",
        "Undecided. Do not assume an answer.",
        &live(&mine, PageType::Open),
        "The question",
    ));

    // Attempts are never current, so they are selected by type alone.
    blocks.push(summary_block(
        "Routes already tried",
        "These failed or were dropped. Do not retry one without a new reason.",
        &of_type(&mine, PageType::Attempt),
        "Why it failed",
    ));

    blocks.push(summary_block(
        "Constraints learned the hard way",
        "Each one cost a debugging session already.",
        &concepts,
        "The constraint",
    ));

    blocks.push(summary_block(
        "Components",
        "Each has its own gotchas page. Read it before touching those files.",
        &live(&mine, PageType::Component),
        "What it does",
    ));

    // One hop out from the overview, in full. The lists above are a map, these
    // are the pages the project page itself says matter most.
    let all: Vec<&Page> = pages.iter().collect();
    for (i, page) in linked_from(overview, &all).iter().enumerate() {
        let heading = if i == 0 {
            "## Pages the overview links to, in full\n\n"
        } else {
            ""
        };
        blocks.push(Block {
            text: format!("{heading}{}\n", page.body),
            pages: 1,
            required: false,
        });
    }

    fill(blocks, project, budget_tokens)
}

/// Take blocks in order until the budget runs out. A required block goes in
/// whatever the budget says, and once one block is dropped the rest follow, so
/// the brief never ends up with a low-priority page and no high-priority one.
fn fill(blocks: Vec<Block>, project: &str, budget_tokens: usize) -> Primer {
    let mut text = String::new();
    let mut included = 0;
    let mut dropped = 0;
    let mut full = false;

    for block in &blocks {
        if block.text.trim().is_empty() {
            continue;
        }
        let fits = estimate_tokens(&text) + estimate_tokens(&block.text) <= budget_tokens;
        if block.required || (!full && fits) {
            text.push_str(&block.text);
            text.push('\n');
            included += block.pages;
        } else {
            full = true;
            dropped += block.pages;
        }
    }

    if dropped > 0 {
        text.push_str(&format!(
            "\n({dropped} more page(s) left out to stay inside the token budget.)\n"
        ));
    }

    Primer {
        project: project.to_string(),
        tokens: estimate_tokens(&text),
        text,
        pages_included: included,
        pages_dropped: dropped,
    }
}

fn header(project: &str, page_count: usize, today: NaiveDate) -> String {
    format!(
        "# {project}: project context\n\n\
         Assembled by Baton on {today} from {page_count} wiki pages.\n\n\
         Every entry comes from a page written by the agent that did the work. A page\n\
         marked superseded or abandoned is kept deliberately: it is history, and the\n\
         reason something is no longer done that way. Each line carries the date its\n\
         page last changed, so a stale claim can be spotted rather than trusted.\n"
    )
}

fn summary_block(heading: &str, note: &str, pages: &[&Page], section: &str) -> Block {
    if pages.is_empty() {
        return Block {
            text: String::new(),
            pages: 0,
            required: false,
        };
    }

    let mut text = format!("## {heading}\n\n{note}\n\n");
    for page in pages {
        let summary = page
            .section(section)
            .map(|s| first_line(&s.body))
            .unwrap_or_default();
        let title = if page.title.is_empty() { &page.id } else { &page.title };
        let status = match page.frontmatter.status {
            Status::Current => String::new(),
            other => format!(" [{}]", other.as_str()),
        };
        text.push_str(&format!(
            "- {title}{status}: {summary} ({})\n",
            page.frontmatter.updated
        ));
    }

    Block {
        text,
        pages: pages.len(),
        required: false,
    }
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

/// Whole pages one hop out from the overview, in the order it links to them.
/// Deliberately includes pages already summarised above: the summary is an
/// index entry, this is the content.
fn linked_from<'a>(overview: Option<&&'a Page>, pages: &[&'a Page]) -> Vec<&'a Page> {
    let Some(overview) = overview else {
        return Vec::new();
    };

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for link in &overview.links {
        if !seen.insert(&link.target) {
            continue;
        }
        if let Some(page) = pages.iter().find(|p| p.id == link.target) {
            out.push(*page);
        }
    }
    out
}

/// Below this a summary reads as a fragment, so a second sentence is pulled in.
/// Deliberately low: one complete sentence is almost always the better summary.
const MIN_SUMMARY_CHARS: usize = 30;

/// The opening of a section, cut at a sentence boundary.
///
/// Pages are hard-wrapped at about 85 columns, so the first physical line is
/// almost always half a sentence. Take the whole first paragraph, then cut on
/// sentences, which is what makes these read as claims rather than fragments.
fn first_line(body: &str) -> String {
    let paragraph: Vec<&str> = body
        .lines()
        .map(str::trim)
        .skip_while(|l| l.is_empty() || l.starts_with("```"))
        .take_while(|l| !l.is_empty() && !l.starts_with("```") && !l.starts_with('#'))
        .collect();

    if paragraph.is_empty() {
        return String::new();
    }

    let text = paragraph
        .join(" ")
        .replace("**", "")
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim()
        .to_string();

    // Whole sentences until there is enough to be worth reading.
    let mut summary = String::new();
    for sentence in split_sentences(&text) {
        if !summary.is_empty() {
            summary.push(' ');
        }
        summary.push_str(sentence.trim());
        if summary.chars().count() >= MIN_SUMMARY_CHARS {
            break;
        }
    }
    let summary = if summary.is_empty() { text } else { summary };
    let summary = summary.trim();

    if summary.chars().count() <= SUMMARY_CHARS {
        return summary.to_string();
    }
    let cut: String = summary.chars().take(SUMMARY_CHARS).collect();
    match cut.rsplit_once(' ') {
        Some((head, _)) => format!("{head}..."),
        None => format!("{cut}..."),
    }
}

/// Split on sentence ends, keeping the punctuation. Deliberately crude: it only
/// has to handle prose a page was written in, and a wrong split costs a slightly
/// long summary rather than a wrong claim.
fn split_sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();

    for (i, &byte) in bytes.iter().enumerate() {
        if byte != b'.' && byte != b'?' && byte != b'!' {
            continue;
        }
        // A period inside `~/Baton/x.md` or `0.13` does not end a sentence.
        match bytes.get(i + 1) {
            Some(b' ') | None => {}
            _ => continue,
        }
        out.push(&text[start..=i]);
        start = i + 1;
    }

    if out.is_empty() {
        return vec![text];
    }
    out
}

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
        let p = assemble(&corpus(), "baton", 10_000, today());

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
        let p = assemble(&corpus(), "baton", 10_000, today());
        assert!(p.text.contains("Markdown files are the source of truth"));
        assert!(
            !p.text.contains("- The content column holds JSON"),
            "a reversed decision must not read as one that still holds"
        );
    }

    #[test]
    fn an_abandoned_route_is_carried_with_the_reason_it_failed() {
        let p = assemble(&corpus(), "baton", 10_000, today());
        assert!(p.text.contains("Hosted proxy [superseded]"));
        assert!(p.text.contains("Every conversation would transit our infrastructure"));
    }

    #[test]
    fn concepts_are_included_even_though_they_belong_to_no_project() {
        let p = assemble(&corpus(), "baton", 10_000, today());
        let summary = p
            .text
            .lines()
            .find(|l| l.starts_with("- Never hold a std Mutex"))
            .expect("a cross-project gotcha must reach the brief");

        // Bold markers are page formatting. They must not survive into a list.
        assert!(!summary.contains("**"));
        assert!(summary.contains("A guard held across an `.await`"));
    }

    #[test]
    fn a_page_the_overview_links_to_is_carried_whole() {
        let p = assemble(&corpus(), "baton", 10_000, today());
        // The one-hop body, not just the one-line summary of the same page.
        assert!(p.text.contains("## The symptom"));
    }

    #[test]
    fn the_budget_drops_from_the_bottom_and_says_so() {
        let full = assemble(&corpus(), "baton", 10_000, today());
        let tight = assemble(&corpus(), "baton", 200, today());

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
        let p = assemble(&only_overview, "baton", 10_000, today());
        assert!(p.text.contains("## Goal"));
        assert!(!p.text.contains("## Open questions"));
        assert!(!p.text.contains("## Routes already tried"));
    }

    #[test]
    fn a_summary_is_whole_sentences_not_the_first_wrapped_line() {
        // Pages are hard-wrapped at about 85 columns, so the first physical
        // line of a section is almost always half a sentence.
        let wrapped = vec![page(
            "projects/baton/decisions/no-model-calls",
            "decision",
            "baton",
            "current",
            "# Baton makes no model calls\n\n## Decision\n\n\
             No API key, no proxy, no rate limit, no cost per use. Every job that\n\
             looked like it needed a model is handled elsewhere.\n\n\
             ## Why\n\nNothing needs inference.\n\n## Rejected\n\nA hosted proxy.\n",
        )];

        let p = assemble(&wrapped, "baton", 10_000, today());
        let summary = p
            .text
            .lines()
            .find(|l| l.starts_with("- Baton makes no model calls"))
            .unwrap();

        assert!(summary.contains("no cost per use."));
        assert!(
            !summary.contains("Every job that"),
            "the summary ran past its first sentence"
        );
    }

    #[test]
    fn a_period_inside_a_path_or_a_version_does_not_end_a_sentence() {
        assert_eq!(
            split_sentences("Pinned to reqwest 0.13 and rustls. Then it built."),
            ["Pinned to reqwest 0.13 and rustls.", " Then it built."]
        );
        assert_eq!(split_sentences("No punctuation here"), ["No punctuation here"]);
    }

    #[test]
    fn the_default_project_is_the_one_touched_last() {
        assert_eq!(most_recent_project(&corpus()).as_deref(), Some("baton"));
        assert_eq!(most_recent_project(&[]), None);
    }
}
