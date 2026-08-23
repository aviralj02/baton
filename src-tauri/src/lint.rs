//! Structural checks over the wiki.
//!
//! The point is not a report. It is that a stale page announces itself *inside
//! the primer*, so the model reading a brief is told which lines to distrust.
//! A report nobody opens is worth nothing.
//!
//! Every check here is free and deterministic — no model, no network. Anything
//! needing judgement (does page A contradict page B?) is deliberately out of
//! scope; the schema calls that out as needing a model and defers it.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::wiki::{Page, PageType, Status};

/// The schema's word ceiling. Over it means the granularity test was skipped.
const MAX_WORDS: usize = 300;

/// Every page type, so a heading can be told apart from a misplaced one.
const EVERY_TYPE: [PageType; 6] = [
    PageType::Project,
    PageType::Decision,
    PageType::Open,
    PageType::Attempt,
    PageType::Component,
    PageType::Gotcha,
];

/// Headings legal on any page type, from `AGENTS.md` § Sections you may add.
const UNIVERSAL: [&str; 2] = ["Open", "Superseded by"];

/// Optional sections a `decision` page may carry beyond its required three.
/// Kept separate from `sections_for` because those are the *required* set, and
/// a missing one is a finding while a missing optional one is not.
const DECISION_OPTIONAL: [&str; 3] = ["Supersedes", "Reverses", "Known cost"];

/// Required sections per type, in the order the schema lists them. The first
/// entry of each is also the "required" set — the schema has no optional
/// per-type headings, so required and allowed coincide.
fn sections_for(t: PageType) -> &'static [&'static str] {
    match t {
        PageType::Project => &["Goal", "Current state", "Next step"],
        PageType::Decision => &["Decision", "Why", "Rejected"],
        PageType::Open => &["The question", "Options", "What it blocks"],
        PageType::Attempt => &["What was tried", "Why it failed", "What it cost"],
        PageType::Component => &["What it does", "Gotchas", "Related files"],
        PageType::Gotcha => &["The constraint", "The symptom", "The fix"],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Finding {
    /// The schema says an empty required section beats a missing one, because
    /// an empty one still says the question was asked.
    MissingSection { heading: String },
    /// Ad-hoc headings are the fastest way to make pages stop resembling each
    /// other, so the set is closed.
    UnknownHeading { heading: String },
    /// A real heading, but not one this page type may carry.
    HeadingOnWrongType { heading: String },
    /// An attempt is a closed thing: `abandoned`, or `superseded` by whatever
    /// replaced it. Never `current`.
    AttemptIsCurrent,
    /// `superseded` without the link makes the chain unfollowable, and the
    /// chain is the whole value.
    SupersededWithoutLink,
    /// Another page claims to supersede this one, but it still reads `current`.
    /// The deterministic half of "two current pages on the same subject".
    ContradictedBySuccessor { successor: String },
    /// Nothing links in and it links nowhere: unreachable by a reader following
    /// the wiki, and invisible to the primer's one-hop pass.
    Orphan,
    /// The skill reads `index.md` to decide whether a subject already has a
    /// page, so an absent page invites a duplicate.
    MissingFromIndex,
    /// Over the schema's ceiling.
    TooLong { words: usize },
    /// A `[[wiki-link]]` pointing at a page that does not exist. Previously a
    /// `broken_links` command that nothing called, so the check never reached
    /// a reader; findings belong where the primer can carry them.
    BrokenLink { target: String },
}

impl Finding {
    /// One line, addressed to whoever is reading the primer — a person or a
    /// model. Says what is wrong, not merely which rule fired.
    pub fn message(&self) -> String {
        match self {
            Finding::MissingSection { heading } => {
                format!("missing its `## {heading}` section")
            }
            Finding::UnknownHeading { heading } => {
                format!("has an off-schema heading `## {heading}`")
            }
            Finding::HeadingOnWrongType { heading } => {
                format!("carries `## {heading}`, which does not belong on this page type")
            }
            Finding::AttemptIsCurrent => {
                "is an attempt marked `current`; an attempt is always closed".into()
            }
            Finding::SupersededWithoutLink => {
                "is superseded but does not link to what replaced it".into()
            }
            Finding::ContradictedBySuccessor { successor } => {
                format!("still reads `current`, but {successor} says it supersedes this")
            }
            Finding::Orphan => "is linked from nowhere and links nowhere".into(),
            Finding::MissingFromIndex => "is not listed in index.md".into(),
            Finding::TooLong { words } => {
                format!("is {words} words, over the {MAX_WORDS}-word limit")
            }
            Finding::BrokenLink { target } => {
                format!("links to [[{target}]], which does not exist")
            }
        }
    }
}

/// Findings keyed by page id. A page with none is absent from the map.
pub type Report = HashMap<String, Vec<Finding>>;

/// Ids named by `[[links]]` in `index.md`. Passed in rather than read here so
/// the checker stays a pure function of its inputs and is trivially testable.
pub fn check(pages: &[Page], indexed_ids: Option<&HashSet<String>>) -> Report {
    let known_ids: HashSet<&str> = pages.iter().map(|p| p.id.as_str()).collect();
    let mut report: Report = HashMap::new();
    let mut add = |id: &str, f: Finding| {
        report.entry(id.to_string()).or_default().push(f);
    };

    // Inbound link counts, and who claims to supersede whom.
    let mut inbound: HashMap<&str, usize> = HashMap::new();
    let mut superseded_by: HashMap<String, String> = HashMap::new();
    for page in pages {
        for link in &page.links {
            *inbound.entry(link.target.as_str()).or_default() += 1;
        }
        // A `## Superseded by` on page X names the page that replaced X. The
        // link therefore points forward, and X is the one that must not be
        // current — checked below via SupersededWithoutLink. The reverse case
        // is a successor naming its predecessor in `## Why`, which is not
        // machine-distinguishable, so only the explicit direction is used.
        if let Some(sec) = section(page, "Superseded by") {
            for link in &page.links {
                if sec.body.contains(&link.target) {
                    superseded_by.insert(page.id.clone(), link.target.clone());
                }
            }
        }
    }

    for page in pages {
        let t = page.frontmatter.page_type;
        let allowed = sections_for(t);

        // Required sections.
        for want in allowed {
            if section(page, want).is_none() {
                add(&page.id, Finding::MissingSection { heading: (*want).into() });
            }
        }

        // Headings outside the schema, and headings from another type.
        for sec in &page.sections {
            let h = sec.heading.trim();
            // Case-insensitive, matching `section()` — otherwise `## goal`
            // satisfies the required-section check and is simultaneously
            // reported as an unknown heading, and that contradiction is pasted
            // into the brief.
            let eq = |k: &&str| k.eq_ignore_ascii_case(h);
            if allowed.iter().any(eq) || UNIVERSAL.iter().any(eq) {
                continue;
            }
            if t == PageType::Decision && DECISION_OPTIONAL.iter().any(eq) {
                continue;
            }
            let known_elsewhere = EVERY_TYPE
                .iter()
                .any(|other| sections_for(*other).iter().any(eq))
                || DECISION_OPTIONAL.iter().any(eq);
            add(
                &page.id,
                if known_elsewhere {
                    Finding::HeadingOnWrongType { heading: h.into() }
                } else {
                    Finding::UnknownHeading { heading: h.into() }
                },
            );
        }

        if t == PageType::Attempt && page.frontmatter.status == Status::Current {
            add(&page.id, Finding::AttemptIsCurrent);
        }

        if page.frontmatter.status == Status::Superseded
            && !superseded_by.contains_key(&page.id)
        {
            add(&page.id, Finding::SupersededWithoutLink);
        }

        // Someone supersedes this page, yet it still claims to be live.
        if page.frontmatter.status == Status::Current {
            if let Some(successor) = superseded_by.get(&page.id) {
                add(
                    &page.id,
                    Finding::ContradictedBySuccessor { successor: successor.clone() },
                );
            }
        }

        let has_inbound = inbound.get(page.id.as_str()).copied().unwrap_or(0) > 0;
        if !has_inbound && page.links.is_empty() {
            add(&page.id, Finding::Orphan);
        }

        if let Some(indexed) = indexed_ids {
            if !indexed.contains(&page.id) {
                add(&page.id, Finding::MissingFromIndex);
            }
        }

        // Links are resolved against the set of real page ids rather than the
        // filesystem: a link can point at a path that exists but is not a page.
        for link in &page.links {
            if !known_ids.contains(link.target.as_str()) {
                add(&page.id, Finding::BrokenLink { target: link.target.clone() });
            }
        }

        let words = page.body.split_whitespace().count();
        if words > MAX_WORDS {
            add(&page.id, Finding::TooLong { words });
        }
    }

    report
}

fn section<'a>(page: &'a Page, heading: &str) -> Option<&'a crate::wiki::Section> {
    page.sections
        .iter()
        .find(|s| s.heading.trim().eq_ignore_ascii_case(heading))
}

/// Ids linked from `index.md`, or `None` when the catalogue cannot be read.
///
/// `None` means "cannot say" and the missing-from-index check is skipped
/// entirely. Returning an empty set instead would mark *every* page as missing,
/// so deleting `index.md` would put a warning about every page into the next
/// brief and tell the model to distrust all of it.
pub fn indexed_ids(root: &std::path::Path) -> Option<HashSet<String>> {
    let text = std::fs::read_to_string(root.join("index.md")).ok()?;
    Some(
        crate::wiki::links_in(root, &text)
            .into_iter()
            .map(|l| l.target)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiki;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("baton-lint-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn page(root: &std::path::Path, rel: &str, body: &str) -> Page {
        let path = root.join(format!("{rel}.md"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        wiki::parse(root, &path, body).unwrap()
    }

    fn fm(t: &str, status: &str) -> String {
        let project = if t == "gotcha" { "null" } else { "baton" };
        format!("---\ntype: {t}\nproject: {project}\nstatus: {status}\nupdated: 2026-08-23\nsources: []\n---\n\n")
    }

    fn all_indexed(pages: &[Page]) -> HashSet<String> {
        pages.iter().map(|p| p.id.clone()).collect()
    }

    #[test]
    fn a_complete_page_produces_no_findings() {
        let root = tmp("clean");
        let a = page(&root, "projects/baton/overview",
            &format!("{}# B\n\n## Goal\n\nx [[concepts/g]]\n\n## Current state\n\ny\n\n## Next step\n\nz\n", fm("project", "current")));
        let b = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![a, b];
        let r = check(&pages, Some(&all_indexed(&pages)));
        assert!(r.is_empty(), "clean wiki produced findings: {r:?}");
    }

    #[test]
    fn an_unreadable_index_skips_the_check_rather_than_flagging_everything() {
        // Returning an empty set here would mark every page as missing from
        // index.md, so deleting the catalogue would put a warning about every
        // page into the next brief and tell the model to distrust all of it.
        let root = tmp("no-index");
        let p = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![p];

        assert!(indexed_ids(&root).is_none(), "no index.md means cannot say");
        let r = check(&pages, None);
        let findings = r.get("concepts/g").cloned().unwrap_or_default();
        assert!(
            !findings.contains(&Finding::MissingFromIndex),
            "flagged every page because the catalogue was unreadable: {findings:?}"
        );
    }

    #[test]
    fn heading_case_is_matched_the_same_way_everywhere() {
        // `section()` matches case-insensitively, so `## goal` counts as
        // present. The allowed-heading check must agree, or one page is both
        // complete and off-schema — and that contradiction reaches the brief.
        let root = tmp("case");
        let p = page(&root, "projects/baton/overview",
            &format!("{}# B\n\n## goal\n\nx\n\n## Current State\n\ny\n\n## next step\n\nz\n", fm("project", "current")));
        let pages = vec![p];
        let findings = check(&pages, Some(&all_indexed(&pages)))
            .get("projects/baton/overview").cloned().unwrap_or_default();
        assert!(
            !findings.iter().any(|f| matches!(f, Finding::UnknownHeading { .. })),
            "case difference reported as an off-schema heading: {findings:?}"
        );
        assert!(
            !findings.iter().any(|f| matches!(f, Finding::MissingSection { .. })),
            "case difference reported as a missing section: {findings:?}"
        );
    }

    #[test]
    fn a_missing_required_section_is_reported() {
        let root = tmp("missing");
        // No `## Rejected` — the section that stops a decision being re-argued.
        let p = page(&root, "projects/baton/decisions/d",
            &format!("{}# D\n\n## Decision\n\nx\n\n## Why\n\ny\n", fm("decision", "current")));
        let pages = vec![p];
        let f = &check(&pages, Some(&all_indexed(&pages)))["projects/baton/decisions/d"];
        assert!(f.contains(&Finding::MissingSection { heading: "Rejected".into() }));
    }

    #[test]
    fn an_off_schema_heading_is_distinguished_from_a_misplaced_one() {
        let root = tmp("headings");
        let p = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n\n## Notes\n\nd\n\n## Rejected\n\ne\n", fm("gotcha", "current")));
        let pages = vec![p];
        let f = &check(&pages, Some(&all_indexed(&pages)))["concepts/g"];
        // `Notes` exists nowhere in the schema.
        assert!(f.contains(&Finding::UnknownHeading { heading: "Notes".into() }));
        // `Rejected` is real, but belongs on a decision.
        assert!(f.contains(&Finding::HeadingOnWrongType { heading: "Rejected".into() }));
    }

    #[test]
    fn a_decisions_optional_sections_are_not_flagged() {
        // AGENTS.md § Sections you may add. Missing these produced false
        // positives on every correctly-written supersede chain.
        let root = tmp("optional");
        let p = page(&root, "projects/baton/decisions/d",
            &format!("{}# D\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n\n## Supersedes\n\n[[projects/baton/decisions/old]]\n\n## Known cost\n\nc\n\n## Reverses\n\n[[projects/baton/decisions/older]]\n", fm("decision", "current")));
        let pages = vec![p];
        let r = check(&pages, Some(&all_indexed(&pages)));
        let findings = r.get("projects/baton/decisions/d").cloned().unwrap_or_default();
        assert!(
            !findings.iter().any(|f| matches!(f, Finding::UnknownHeading { .. })),
            "optional decision sections flagged: {findings:?}"
        );
    }

    #[test]
    fn an_optional_decision_heading_on_another_type_is_still_wrong() {
        let root = tmp("optional-wrong");
        let p = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n\n## Known cost\n\nd\n", fm("gotcha", "current")));
        let pages = vec![p];
        let f = &check(&pages, Some(&all_indexed(&pages)))["concepts/g"];
        assert!(f.contains(&Finding::HeadingOnWrongType { heading: "Known cost".into() }));
    }

    #[test]
    fn a_link_to_a_page_that_does_not_exist_is_flagged() {
        let root = tmp("broken-link");
        let p = page(&root, "projects/baton/overview",
            &format!("{}# B\n\n## Goal\n\nSee [[concepts/nope]].\n\n## Current state\n\ny\n\n## Next step\n\nz\n", fm("project", "current")));
        let pages = vec![p];
        let f = &check(&pages, Some(&all_indexed(&pages)))["projects/baton/overview"];
        assert!(f.contains(&Finding::BrokenLink { target: "concepts/nope".into() }));
    }

    #[test]
    fn a_link_to_a_real_page_is_not_flagged() {
        let root = tmp("good-link");
        let a = page(&root, "projects/baton/overview",
            &format!("{}# B\n\n## Goal\n\nSee [[concepts/g]].\n\n## Current state\n\ny\n\n## Next step\n\nz\n", fm("project", "current")));
        let b = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![a, b];
        let r = check(&pages, Some(&all_indexed(&pages)));
        let f = r.get("projects/baton/overview").cloned().unwrap_or_default();
        assert!(!f.iter().any(|x| matches!(x, Finding::BrokenLink { .. })), "{f:?}");
    }

    #[test]
    fn an_attempt_may_not_be_current() {
        let root = tmp("attempt");
        let p = page(&root, "projects/baton/attempts/a",
            &format!("{}# A\n\n## What was tried\n\nx\n\n## Why it failed\n\ny\n\n## What it cost\n\nz\n", fm("attempt", "current")));
        let pages = vec![p];
        assert!(check(&pages, Some(&all_indexed(&pages)))["projects/baton/attempts/a"]
            .contains(&Finding::AttemptIsCurrent));
    }

    #[test]
    fn superseded_without_a_forward_link_breaks_the_chain() {
        let root = tmp("chain");
        let p = page(&root, "projects/baton/decisions/old",
            &format!("{}# Old\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n", fm("decision", "superseded")));
        let pages = vec![p];
        assert!(check(&pages, Some(&all_indexed(&pages)))["projects/baton/decisions/old"]
            .contains(&Finding::SupersededWithoutLink));
    }

    #[test]
    fn a_page_its_own_successor_replaced_should_not_read_current() {
        let root = tmp("contradict");
        // `old` says it was replaced by `new`, but is still marked current.
        let old = page(&root, "projects/baton/decisions/old",
            &format!("{}# Old\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n\n## Superseded by\n\n[[projects/baton/decisions/new]]\n", fm("decision", "current")));
        let new = page(&root, "projects/baton/decisions/new",
            &format!("{}# New\n\n## Decision\n\nx\n\n## Why\n\n[[projects/baton/decisions/old]]\n\n## Rejected\n\nz\n", fm("decision", "current")));
        let pages = vec![old, new];
        let f = &check(&pages, Some(&all_indexed(&pages)))["projects/baton/decisions/old"];
        assert!(f.iter().any(|x| matches!(x, Finding::ContradictedBySuccessor { .. })),
            "a page replaced by another must not read current: {f:?}");
    }

    #[test]
    fn an_unreachable_page_is_an_orphan() {
        let root = tmp("orphan");
        let a = page(&root, "projects/baton/overview",
            &format!("{}# B\n\n## Goal\n\nx\n\n## Current state\n\ny\n\n## Next step\n\nz\n", fm("project", "current")));
        let b = page(&root, "concepts/lonely",
            &format!("{}# L\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![a, b];
        let r = check(&pages, Some(&all_indexed(&pages)));
        // Neither links anywhere, so both are unreachable.
        assert!(r["concepts/lonely"].contains(&Finding::Orphan));
        assert!(r["projects/baton/overview"].contains(&Finding::Orphan));
    }

    #[test]
    fn a_page_absent_from_the_index_invites_a_duplicate() {
        let root = tmp("index");
        let p = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![p];
        // Index lists nothing.
        let f = &check(&pages, Some(&HashSet::new()))["concepts/g"];
        assert!(f.contains(&Finding::MissingFromIndex));
    }

    #[test]
    fn a_page_over_the_word_limit_is_flagged() {
        let root = tmp("long");
        let filler = "word ".repeat(400);
        let p = page(&root, "concepts/g",
            &format!("{}# G\n\n## The constraint\n\n{filler}\n\n## The symptom\n\nb\n\n## The fix\n\nc\n", fm("gotcha", "current")));
        let pages = vec![p];
        let f = &check(&pages, Some(&all_indexed(&pages)))["concepts/g"];
        assert!(f.iter().any(|x| matches!(x, Finding::TooLong { .. })));
    }

    #[test]
    fn every_finding_reads_as_a_sentence_about_the_page() {
        // The primer pastes these into a model's context, so a bare enum name
        // or a rule id would be useless there.
        for f in [
            Finding::MissingSection { heading: "Goal".into() },
            Finding::UnknownHeading { heading: "Notes".into() },
            Finding::HeadingOnWrongType { heading: "Rejected".into() },
            Finding::AttemptIsCurrent,
            Finding::SupersededWithoutLink,
            Finding::ContradictedBySuccessor { successor: "a/b".into() },
            Finding::Orphan,
            Finding::MissingFromIndex,
            Finding::TooLong { words: 400 },
        ] {
            let m = f.message();
            assert!(m.len() > 15, "{f:?} message too terse: {m}");
            assert!(!m.contains("Finding"), "{m} leaks the type name");
        }
    }
}



