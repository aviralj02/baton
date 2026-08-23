//! Regenerates `~/Baton/index.md` from the tree.
//!
//! The index is what the `/baton` skill reads to decide whether a subject
//! already has a page. A stale index therefore causes the exact duplication the
//! schema exists to prevent — the agent sees no page on a subject, writes a
//! second one, and the two drift apart.
//!
//! Derived, never hand-maintained: the tree is the truth, so this is a pure
//! function of the indexed pages.

use std::collections::BTreeMap;

use crate::wiki::{Page, PageType, Status};

/// Order sections deterministically. The project page first because that is
/// what a reader wants; gotchas last because they belong to no project.
fn type_rank(t: PageType) -> u8 {
    match t {
        PageType::Project => 0,
        PageType::Component => 1,
        PageType::Decision => 2,
        PageType::Open => 3,
        PageType::Attempt => 4,
        PageType::Gotcha => 5,
    }
}

fn type_heading(t: PageType) -> &'static str {
    match t {
        PageType::Project => "Overview",
        PageType::Component => "Components",
        PageType::Decision => "Decisions",
        PageType::Open => "Open questions",
        PageType::Attempt => "Attempts",
        PageType::Gotcha => "Gotchas",
    }
}

/// The half-line of description the schema asks for.
///
/// Uses the page's own first sentence rather than inventing a summary: this
/// runs with no model, and a wrong summary is worse than a terse one.
fn blurb(page: &Page) -> String {
    let source = page
        .sections
        .first()
        .map(|s| s.body.as_str())
        .unwrap_or(&page.preamble);

    // Join the first paragraph before looking for a sentence. Markdown wraps
    // prose across lines, so reading one line truncates mid-sentence.
    let paragraph = source
        .split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with("```") && !p.starts_with('-'))
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");

    // Cut at the first sentence end, then hard-cap so one runaway paragraph
    // cannot make the index unreadable.
    let sentence = paragraph
        .split_once(". ")
        .map(|(a, _)| a)
        .unwrap_or(&paragraph);
    let sentence = sentence.trim_end_matches('.').trim();

    if sentence.chars().count() > 100 {
        let cut: String = sentence.chars().take(97).collect();
        format!("{}…", cut.trim_end())
    } else {
        sentence.to_string()
    }
}

/// A last-resort label for a page with no title: its final path segment, with
/// hyphens opened out. `projects/baton/decisions/files-are-truth` becomes
/// "Files are truth" — still not a sentence, but readable rather than a path.
fn readable_id(id: &str) -> String {
    let last = id.rsplit('/').next().unwrap_or(id).replace(['-', '_'], " ");
    let mut c = last.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => last,
    }
}

/// A marker for pages the reader should not treat as live.
fn status_note(status: Status) -> &'static str {
    match status {
        Status::Current => "",
        Status::Superseded => " — superseded",
        Status::Abandoned => " — abandoned",
        Status::Stale => " — stale",
    }
}

/// Build the whole file. `pages` may arrive in any order.
pub fn render(pages: &[Page]) -> String {
    let mut out = String::from(
        "# Baton index\n\n\
         Every page, grouped by project and then by type. Derived from the tree — \
         edit the tree, not this file.\n",
    );

    if pages.is_empty() {
        out.push_str(
            "\n_No pages yet. Run `/baton` at the end of a working session and the \
             agent will propose the first ones._\n",
        );
        return out;
    }

    // BTreeMap so the output is stable across runs; an index that reorders
    // itself every save produces noisy diffs and looks broken in git.
    let mut by_project: BTreeMap<String, Vec<&Page>> = BTreeMap::new();
    for page in pages {
        // Pages in concepts/ belong to no project and are grouped last, which
        // the leading tilde achieves without a special case in the loop.
        let key = page
            .frontmatter
            .project
            .clone()
            .unwrap_or_else(|| "~concepts".to_string());
        by_project.entry(key).or_default().push(page);
    }

    for (project, mut group) in by_project {
        let heading = if project == "~concepts" {
            "Concepts".to_string()
        } else {
            project.clone()
        };
        out.push_str(&format!("\n## {heading}\n"));

        group.sort_by(|a, b| {
            type_rank(a.frontmatter.page_type)
                .cmp(&type_rank(b.frontmatter.page_type))
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut current_type: Option<PageType> = None;
        for page in group {
            let t = page.frontmatter.page_type;
            if current_type != Some(t) {
                out.push_str(&format!("\n### {}\n\n", type_heading(t)));
                current_type = Some(t);
            }
            let note = status_note(page.frontmatter.status);
            let text = blurb(page);
            // `[[id|Title]]` keeps the path as the link target while showing the
            // claim. A bare `[[id]]` renders as machinery, and this file is read
            // by a person deciding what already exists.
            let label = if page.title.trim().is_empty() {
                readable_id(&page.id)
            } else {
                page.title.trim().to_string()
            };
            if text.is_empty() {
                out.push_str(&format!("- [[{}|{label}]]{note}\n", page.id));
            } else {
                out.push_str(&format!("- [[{}|{label}]] — {text}{note}\n", page.id));
            }
        }
    }

    out
}

/// Write the index only when it changed.
///
/// Unconditional writes would retrigger the file watcher, which would reindex,
/// which would rewrite the index — a loop that never settles.
pub fn write_if_changed(root: &std::path::Path, pages: &[Page]) -> std::io::Result<bool> {
    let path = root.join("index.md");
    let next = render(pages);
    if std::fs::read_to_string(&path).ok().as_deref() == Some(next.as_str()) {
        return Ok(false);
    }
    std::fs::write(&path, next)?;
    Ok(true)
}

/// Walk the wiki and rewrite `index.md` if it changed.
///
/// Walks rather than reusing the indexer's output because `db::sync` is
/// incremental: it only reparses files whose mtime moved, so it never holds the
/// whole tree. The index must list every page, not the changed ones.
pub fn regenerate(root: &std::path::Path) -> std::io::Result<bool> {
    let pages: Vec<crate::wiki::Page> = crate::wiki::walk(root)
        .unwrap_or_default()
        .iter()
        // A page that fails to parse is left out rather than aborting the whole
        // index — one malformed file should not blank the table of contents.
        .filter_map(|path| crate::wiki::read(root, path).ok())
        .collect();
    write_if_changed(root, &pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiki;

    fn page(root: &std::path::Path, rel: &str, body: &str) -> Page {
        let path = root.join(format!("{rel}.md"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        wiki::parse(root, &path, body).unwrap()
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("baton-index-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const OVERVIEW: &str = "---\ntype: project\nproject: baton\nstatus: current\nupdated: 2026-08-23\nsources: []\n---\n\n# Baton\n\n## Goal\n\nA central markdown wiki. It holds everything.\n\n## Current state\n\nPhase 1.\n\n## Next step\n\nShip.\n";
    const GOTCHA: &str = "---\ntype: gotcha\nproject: null\nstatus: current\nupdated: 2026-08-23\nsources: []\n---\n\n# AppKit main thread\n\n## The constraint\n\nEvery AppKit window mutation must happen on the main thread.\n\n## The symptom\n\nEXC_BREAKPOINT.\n\n## The fix\n\nHop threads.\n";
    const OLD_DECISION: &str = "---\ntype: decision\nproject: baton\nstatus: superseded\nupdated: 2026-08-23\nsources: []\n---\n\n# JSON not markdown\n\n## Decision\n\nStore JSON in SQLite.\n\n## Why\n\nOne flat document.\n\n## Rejected\n\nMarkdown files.\n";

    #[test]
    fn groups_by_project_with_concepts_last() {
        let root = tmp("group");
        let pages = vec![
            page(&root, "concepts/appkit", GOTCHA),
            page(&root, "projects/baton/overview", OVERVIEW),
        ];
        let out = render(&pages);
        assert!(
            out.find("## baton").unwrap() < out.find("## Concepts").unwrap(),
            "concepts must sort after real projects"
        );
        // The link target is the id; what is shown is the title.
        assert!(out.contains("[[projects/baton/overview|"));
        assert!(out.contains("[[concepts/appkit|"));
    }

    #[test]
    fn the_index_shows_titles_not_paths() {
        let root = tmp("titles");
        let out = render(&[page(&root, "projects/baton/overview", OVERVIEW)]);
        assert!(
            out.contains("[[projects/baton/overview|Baton]]"),
            "the path is shown instead of the title: {out}"
        );
    }

    #[test]
    fn a_titleless_page_falls_back_to_a_readable_name() {
        assert_eq!(readable_id("projects/baton/decisions/files-are-truth"), "Files are truth");
        assert_eq!(readable_id("concepts/appkit_main_thread"), "Appkit main thread");
    }

    #[test]
    fn every_page_appears() {
        let root = tmp("every");
        let pages = vec![
            page(&root, "projects/baton/overview", OVERVIEW),
            page(&root, "projects/baton/decisions/json", OLD_DECISION),
            page(&root, "concepts/appkit", GOTCHA),
        ];
        let out = render(&pages);
        // The schema requires it, and the skill relies on it to avoid writing
        // a duplicate page.
        for p in &pages {
            assert!(out.contains(&format!("[[{}|", p.id)), "{} missing", p.id);
        }
    }

    #[test]
    fn non_current_pages_are_marked() {
        let root = tmp("status");
        let out = render(&[page(&root, "projects/baton/decisions/json", OLD_DECISION)]);
        assert!(out.contains("superseded"), "a reader would think it is live");
    }

    #[test]
    fn blurb_comes_from_the_page_not_invented() {
        let root = tmp("blurb");
        let out = render(&[page(&root, "projects/baton/overview", OVERVIEW)]);
        assert!(out.contains("A central markdown wiki"));
    }

    #[test]
    fn blurb_is_a_whole_sentence_across_wrapped_lines() {
        // Markdown wraps prose, so a line-based blurb truncates mid-sentence.
        let root = tmp("wrapped");
        let wrapped = "---\ntype: project\nproject: w\nstatus: current\nupdated: 2026-08-23\nsources: []\n---\n\n# W\n\n## Goal\n\nA central markdown wiki, written by the agent\nthat did the work and read back later.\n\n## Current state\n\nx\n\n## Next step\n\ny\n";
        let out = render(&[page(&root, "projects/w/overview", wrapped)]);
        assert!(
            out.contains("read back later"),
            "blurb stopped at the line break: {out}"
        );
        assert!(!out.contains("\n  that did"), "line break leaked into the index");
    }

    #[test]
    fn output_is_stable_across_runs() {
        // An index that reorders itself every save makes noisy diffs and looks
        // broken in git.
        let root = tmp("stable");
        let pages = vec![
            page(&root, "concepts/appkit", GOTCHA),
            page(&root, "projects/baton/overview", OVERVIEW),
        ];
        let mut reversed = pages.clone();
        reversed.reverse();
        assert_eq!(render(&pages), render(&reversed));
    }

    #[test]
    fn write_is_skipped_when_nothing_changed() {
        // An unconditional write retriggers the watcher, which reindexes, which
        // rewrites the index — a loop that never settles.
        let root = tmp("loop");
        let pages = vec![page(&root, "projects/baton/overview", OVERVIEW)];
        assert!(write_if_changed(&root, &pages).unwrap(), "first write");
        assert!(!write_if_changed(&root, &pages).unwrap(), "rewrote unchanged");
    }

    #[test]
    fn empty_wiki_says_so() {
        let root = tmp("empty");
        let out = render(&[]);
        assert!(out.contains("No pages yet"));
        assert!(write_if_changed(&root, &[]).unwrap());
    }
}
