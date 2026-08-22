//! Reading the markdown wiki at `~/Baton/`.
//!
//! The files are the source of truth. Nothing in here writes: pages are written
//! by the agent that did the work, and by hand. This module turns one page into
//! the shape the search index and the primer need.
//!
//! The parsers are deliberately strict about the schema in `~/Baton/AGENTS.md`
//! and lenient about whitespace, because a human editing a page in Obsidian is
//! an expected author. Anything it cannot understand is an error naming the file
//! and the field, never a silent default.

use std::path::{Component, Path, PathBuf};

use chrono::NaiveDate;
use serde::Serialize;

/// Markdown files in the wiki that are not pages. `index.md` and `log.md` are
/// derived, `AGENTS.md` is the schema. None of the three carry frontmatter.
const NON_PAGES: [&str; 3] = ["AGENTS", "index", "log"];

#[derive(Debug, thiserror::Error)]
pub enum WikiError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is outside the wiki root")]
    OutsideRoot { path: String },
    #[error("{path} has no frontmatter: a page must start with a --- line")]
    MissingFrontmatter { path: String },
    #[error("{path} has an unterminated frontmatter block")]
    UnterminatedFrontmatter { path: String },
    #[error("{path} is missing the required frontmatter field `{key}`")]
    MissingField { path: String, key: String },
    #[error("{path} has an invalid `{key}`: {detail}")]
    BadField {
        path: String,
        key: String,
        detail: String,
    },
}

impl serde::Serialize for WikiError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, WikiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PageType {
    Project,
    Decision,
    Open,
    Attempt,
    Component,
    Gotcha,
}

impl PageType {
    const ALL: [(&'static str, PageType); 6] = [
        ("project", PageType::Project),
        ("decision", PageType::Decision),
        ("open", PageType::Open),
        ("attempt", PageType::Attempt),
        ("component", PageType::Component),
        ("gotcha", PageType::Gotcha),
    ];

    pub fn as_str(self) -> &'static str {
        Self::ALL.iter().find(|(_, t)| *t == self).unwrap().0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Current,
    Superseded,
    Abandoned,
    Stale,
}

impl Status {
    const ALL: [(&'static str, Status); 4] = [
        ("current", Status::Current),
        ("superseded", Status::Superseded),
        ("abandoned", Status::Abandoned),
        ("stale", Status::Stale),
    ];

    pub fn as_str(self) -> &'static str {
        Self::ALL.iter().find(|(_, s)| *s == self).unwrap().0
    }
}

/// The five required frontmatter fields. Unknown keys are ignored rather than
/// rejected, so a human addition does not break the read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frontmatter {
    #[serde(rename = "type")]
    pub page_type: PageType,
    /// `None` for pages in `concepts/`, which belong to no project.
    pub project: Option<String>,
    pub status: Status,
    pub updated: NaiveDate,
    pub sources: Vec<String>,
}

/// One `##` heading and everything under it, including any deeper headings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub heading: String,
    pub body: String,
}

/// A `[[wiki-link]]` and the file it points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiLink {
    /// The page id the link names: root-relative, no `.md`, no anchor.
    pub target: String,
    /// The `[[target|alias]]` display text, when the link carries one.
    pub alias: Option<String>,
    /// Absolute path to the target file. `None` when the target escapes the
    /// wiki root, which is malformed rather than merely broken. The file is not
    /// checked for existence here, so a resolved path may still not exist.
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// Root-relative path without `.md`, forward slashes on every platform.
    /// This is what a `[[wiki-link]]` names and what the index keys on.
    pub id: String,
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    /// The `#` heading. Empty when the page has none.
    pub title: String,
    /// Anything between the title and the first `##`. Usually empty.
    pub preamble: String,
    pub sections: Vec<Section>,
    /// Every link in the body, in the order they appear, duplicates included.
    pub links: Vec<WikiLink>,
    /// The whole body after the frontmatter. Kept for full-text indexing.
    pub body: String,
}

impl Page {
    /// Look a section up by heading, ignoring case. The schema fixes the
    /// spelling, but a hand-edited page may not.
    pub fn section(&self, heading: &str) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.heading.eq_ignore_ascii_case(heading))
    }
}

/// Read and parse one page.
pub fn read(root: &Path, path: &Path) -> Result<Page> {
    let text = std::fs::read_to_string(path).map_err(|source| WikiError::Io {
        path: display(path),
        source,
    })?;
    parse(root, path, &text)
}

/// Every page under `root`, sorted. Skips dotfiles, non-markdown files, and the
/// three markdown files that are not pages.
pub fn walk(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_into(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_into(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| WikiError::Io {
        path: display(dir),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| WikiError::Io {
            path: display(dir),
            source,
        })?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            walk_into(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if NON_PAGES.contains(&page_id(root, &path)?.as_str()) {
            continue;
        }
        out.push(path);
    }
    Ok(())
}

pub fn parse(root: &Path, path: &Path, text: &str) -> Result<Page> {
    let id = page_id(root, path)?;
    let (yaml, body) = split_frontmatter(text, &id)?;
    let frontmatter = parse_frontmatter(yaml, &id)?;
    let (title, preamble, sections) = parse_body(body);

    Ok(Page {
        id,
        path: path.to_path_buf(),
        frontmatter,
        title,
        preamble,
        sections,
        links: links_in(root, body),
        body: body.trim().to_string(),
    })
}

/// Root-relative path without the `.md`, always forward-slashed so the same
/// page has the same id on macOS and Windows.
pub fn page_id(root: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| WikiError::OutsideRoot { path: display(path) })?;

    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => return Err(WikiError::OutsideRoot { path: display(path) }),
        }
    }

    let joined = parts.join("/");
    Ok(joined.strip_suffix(".md").unwrap_or(&joined).to_string())
}

// ------------------------------------------------------------- frontmatter

/// Split the leading `---` block off the body. Returns (yaml, body).
fn split_frontmatter<'a>(text: &'a str, id: &str) -> Result<(&'a str, &'a str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    let mut offset = 0;
    let mut yaml_start = None;
    for raw in text.split_inclusive('\n') {
        let line = raw.trim_end_matches('\n').trim_end_matches('\r').trim();
        let next = offset + raw.len();

        match yaml_start {
            None => {
                if line != "---" {
                    return Err(WikiError::MissingFrontmatter { path: id.to_string() });
                }
                yaml_start = Some(next);
            }
            // `...` is YAML's other terminator. Obsidian writes `---`.
            Some(start) if line == "---" || line == "..." => {
                return Ok((&text[start..offset], &text[next..]));
            }
            Some(_) => {}
        }
        offset = next;
    }

    match yaml_start {
        Some(_) => Err(WikiError::UnterminatedFrontmatter { path: id.to_string() }),
        None => Err(WikiError::MissingFrontmatter { path: id.to_string() }),
    }
}

/// One frontmatter value. The schema has no nested maps, so a scalar and a list
/// of scalars is the whole grammar this needs to cover.
enum Value {
    Scalar(String),
    List(Vec<String>),
}

/// Parses the subset of YAML the schema allows: flat `key: value` pairs whose
/// values are scalars, `null`, flow sequences (`[a, b]`) or block sequences.
/// A full YAML parser would accept anchors, nested maps and block scalars, none
/// of which a page may contain, and would turn a typo into a valid document
/// instead of an error.
fn parse_frontmatter(yaml: &str, id: &str) -> Result<Frontmatter> {
    let mut fields: Vec<(String, Value)> = Vec::new();
    let mut lines = yaml
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .peekable();

    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, rest)) = line.split_once(':') else {
            return Err(WikiError::BadField {
                path: id.to_string(),
                key: key_hint(line),
                detail: "expected `key: value`".to_string(),
            });
        };

        let key = key.trim().to_string();
        let value = strip_comment(rest.trim());

        if value.is_empty() {
            // Either a block sequence on the following lines, or an empty value.
            let mut items = Vec::new();
            while let Some(next) = lines.peek() {
                let next = next.trim();
                if next.is_empty() || next.starts_with('#') {
                    lines.next();
                } else if let Some(item) = next.strip_prefix("- ") {
                    items.push(unquote(strip_comment(item.trim())));
                    lines.next();
                } else {
                    break;
                }
            }
            fields.push((key, Value::List(items)));
        } else if let Some(inner) = value.strip_prefix('[') {
            let inner = inner.strip_suffix(']').unwrap_or(inner);
            let items = inner
                .split(',')
                .map(|item| unquote(item.trim()))
                .filter(|item| !item.is_empty())
                .collect();
            fields.push((key, Value::List(items)));
        } else {
            fields.push((key, Value::Scalar(unquote(value))));
        }
    }

    let find = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    let scalar = |key: &'static str| -> Result<String> {
        match find(key) {
            Some(Value::Scalar(s)) => Ok(s.clone()),
            // An empty value parses as an empty list. Read it back as "".
            Some(Value::List(items)) if items.is_empty() => Ok(String::new()),
            Some(Value::List(_)) => Err(WikiError::BadField {
                path: id.to_string(),
                key: key.to_string(),
                detail: "expected a single value, found a list".to_string(),
            }),
            None => Err(WikiError::MissingField {
                path: id.to_string(),
                key: key.to_string(),
            }),
        }
    };

    let raw_type = scalar("type")?;
    let page_type = PageType::ALL
        .iter()
        .find(|(name, _)| *name == raw_type)
        .map(|(_, t)| *t)
        .ok_or_else(|| WikiError::BadField {
            path: id.to_string(),
            key: "type".to_string(),
            detail: format!(
                "`{}` is not one of {}",
                raw_type,
                PageType::ALL.map(|(n, _)| n).join(" | ")
            ),
        })?;

    let raw_status = scalar("status")?;
    let status = Status::ALL
        .iter()
        .find(|(name, _)| *name == raw_status)
        .map(|(_, s)| *s)
        .ok_or_else(|| WikiError::BadField {
            path: id.to_string(),
            key: "status".to_string(),
            detail: format!(
                "`{}` is not one of {}",
                raw_status,
                Status::ALL.map(|(n, _)| n).join(" | ")
            ),
        })?;

    let raw_updated = scalar("updated")?;
    let updated = NaiveDate::parse_from_str(&raw_updated, "%Y-%m-%d").map_err(|_| {
        WikiError::BadField {
            path: id.to_string(),
            key: "updated".to_string(),
            detail: format!("`{raw_updated}` is not an ISO date like 2026-08-22"),
        }
    })?;

    let raw_project = scalar("project")?;
    let project = match raw_project.as_str() {
        "" | "null" | "~" | "Null" | "NULL" => None,
        other => Some(other.to_string()),
    };

    let sources = match find("sources") {
        Some(Value::List(items)) => items.clone(),
        // A lone unbracketed id, which the schema does not show but reads fine.
        Some(Value::Scalar(one)) => vec![one.clone()],
        None => {
            return Err(WikiError::MissingField {
                path: id.to_string(),
                key: "sources".to_string(),
            })
        }
    };

    Ok(Frontmatter {
        page_type,
        project,
        status,
        updated,
        sources,
    })
}

/// Drop a trailing ` # comment`, which the schema block in AGENTS.md uses. A
/// `#` inside quotes is part of the value.
fn strip_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut quote: Option<u8> = None;

    for (i, &byte) in bytes.iter().enumerate() {
        match byte {
            b'"' | b'\'' if quote == Some(byte) => quote = None,
            b'"' | b'\'' if quote.is_none() => quote = Some(byte),
            b'#' if quote.is_none() && (i == 0 || bytes[i - 1] == b' ') => {
                return value[..i].trim_end();
            }
            _ => {}
        }
    }
    value
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn key_hint(line: &str) -> String {
    line.chars().take(40).collect()
}

// -------------------------------------------------------------------- body

/// Tracks fenced code blocks, so a `##` or a `[[link]]` inside a fence is read
/// as sample text rather than as markup. Component pages fence their
/// `## Related files` list, and any page may quote the schema.
#[derive(Default)]
struct Fence {
    open: Option<(char, usize)>,
}

impl Fence {
    /// Feed one line. Returns true when the line is a fence delimiter or sits
    /// inside a fence, meaning it must not be parsed as markdown.
    fn feed(&mut self, line: &str) -> bool {
        match self.open {
            None => match fence_run(line) {
                Some(run) => {
                    self.open = Some(run);
                    true
                }
                None => false,
            },
            Some((open_char, open_len)) => {
                if let Some((c, len)) = fence_run(line) {
                    let rest = line.trim_start().trim_start_matches(c).trim();
                    if c == open_char && len >= open_len && rest.is_empty() {
                        self.open = None;
                    }
                }
                true
            }
        }
    }
}

fn fence_run(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let len = trimmed.chars().take_while(|c| *c == first).count();
    (len >= 3).then_some((first, len))
}

/// Level and text of an ATX heading. Four spaces of indent makes it a code
/// block, not a heading.
fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    if line.len() - trimmed.len() >= 4 {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim()))
}

/// Split a body into its title, its preamble and its `##` sections. Deeper
/// headings stay inside the section that contains them, which is what keeps
/// `index.md`-shaped pages from exploding into one section per sub-heading.
fn parse_body(body: &str) -> (String, String, Vec<Section>) {
    let mut title = String::new();
    let mut preamble: Vec<&str> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;
    let mut fence = Fence::default();

    for line in body.lines() {
        let line = line.trim_end_matches('\r');

        if !fence.feed(line) {
            match heading(line) {
                Some((1, text)) if title.is_empty() && current.is_none() => {
                    title = text.to_string();
                    continue;
                }
                Some((2, text)) => {
                    if let Some((heading, lines)) = current.take() {
                        sections.push(Section::new(heading, &lines));
                    }
                    current = Some((text.to_string(), Vec::new()));
                    continue;
                }
                _ => {}
            }
        }

        match current.as_mut() {
            Some((_, lines)) => lines.push(line),
            None => preamble.push(line),
        }
    }

    if let Some((heading, lines)) = current {
        sections.push(Section::new(heading, &lines));
    }

    (title, preamble.join("\n").trim().to_string(), sections)
}

impl Section {
    fn new(heading: String, lines: &[&str]) -> Self {
        Section {
            heading,
            body: lines.join("\n").trim().to_string(),
        }
    }
}

// ------------------------------------------------------------------- links

/// Every `[[wiki-link]]` in `text`, in order. Links inside fenced code blocks
/// and inline code spans are examples, not edges, so they are skipped.
pub fn links_in(root: &Path, text: &str) -> Vec<WikiLink> {
    let mut out = Vec::new();
    let mut fence = Fence::default();

    for line in text.lines() {
        if fence.feed(line.trim_end_matches('\r')) {
            continue;
        }
        scan_line(root, line, &mut out);
    }
    out
}

fn scan_line(root: &Path, line: &str, out: &mut Vec<WikiLink>) {
    let bytes = line.as_bytes();
    let mut code: Option<usize> = None;
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                let run = bytes[i..].iter().take_while(|b| **b == b'`').count();
                // A span closes only on a backtick run of the same length.
                code = match code {
                    Some(open) if open == run => None,
                    open @ Some(_) => open,
                    None => Some(run),
                };
                i += run;
            }
            b'[' if code.is_none() && bytes.get(i + 1) == Some(&b'[') => {
                let rest = &line[i + 2..];
                match (rest.find("]]"), rest.find("[[")) {
                    // An unclosed `[[` must not swallow the link after it.
                    (Some(close), Some(open)) if open < close => i += open + 2,
                    (Some(close), _) => {
                        if let Some(link) = parse_link(root, &rest[..close]) {
                            out.push(link);
                        }
                        i += close + 4;
                    }
                    (None, _) => i += 2,
                }
            }
            _ => i += 1,
        }
    }
}

fn parse_link(root: &Path, inner: &str) -> Option<WikiLink> {
    let (target, alias) = match inner.split_once('|') {
        Some((target, alias)) => (target, Some(alias.trim().to_string())),
        None => (inner, None),
    };

    // Obsidian accepts a `#heading` anchor and a written-out `.md`. Neither
    // changes which page is meant.
    let target = target.split('#').next().unwrap_or_default().trim();
    let target = target.trim_start_matches("./").trim_start_matches('/');
    let target = target.strip_suffix(".md").unwrap_or(target);
    let target = target.trim_end_matches('/');
    if target.is_empty() {
        return None;
    }

    Some(WikiLink {
        path: page_path(root, target),
        target: target.to_string(),
        alias: alias.filter(|a| !a.is_empty()),
    })
}

/// The file a page id names. `None` when the id climbs out of the root, which
/// is the guard on any id that arrives from the webview or from a link.
pub fn page_path(root: &Path, target: &str) -> Option<PathBuf> {
    let relative = Path::new(target);
    if relative.is_absolute() {
        return None;
    }
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    Some(root.join(format!("{target}.md")))
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/wiki";

    fn root() -> PathBuf {
        PathBuf::from(ROOT)
    }

    fn parse_page(text: &str) -> Page {
        parse(&root(), &root().join("projects/baton/db.md"), text).unwrap()
    }

    fn page(frontmatter: &str, body: &str) -> String {
        format!("---\n{}\n---\n\n{}", frontmatter.trim(), body.trim_start())
    }

    const FM: &str = "
type: component
project: baton
status: current
updated: 2026-08-22
sources: [7123f71b]";

    #[test]
    fn parses_the_shape_a_real_page_has() {
        let p = parse_page(&page(
            FM,
            "# db.rs, the SQLite layer\n\n\
             ## What it does\n\nOwns the only connection.\n\n\
             ## Gotchas\n\n- Never hold the mutex across an await.\n\n\
             ## Related files\n\n```\nsrc-tauri/src/db.rs\n```\n",
        ));

        assert_eq!(p.id, "projects/baton/db");
        assert_eq!(p.frontmatter.page_type, PageType::Component);
        assert_eq!(p.frontmatter.project.as_deref(), Some("baton"));
        assert_eq!(p.frontmatter.status, Status::Current);
        assert_eq!(p.frontmatter.updated.to_string(), "2026-08-22");
        assert_eq!(p.frontmatter.sources, ["7123f71b"]);
        assert_eq!(p.title, "db.rs, the SQLite layer");
        assert!(p.preamble.is_empty());

        let headings: Vec<&str> = p.sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["What it does", "Gotchas", "Related files"]);
        assert_eq!(
            p.section("what it does").unwrap().body,
            "Owns the only connection."
        );
        // The fenced block is the section's content, not three lost lines.
        assert_eq!(
            p.section("Related files").unwrap().body,
            "```\nsrc-tauri/src/db.rs\n```"
        );
    }

    #[test]
    fn concepts_pages_carry_a_null_project() {
        let p = parse_page(&page(
            "type: gotcha\nproject: null\nstatus: current\nupdated: 2026-08-22\nsources: [7123f71b]",
            "# Never hold a std Mutex across an await\n",
        ));
        assert!(p.frontmatter.project.is_none());
        assert_eq!(p.frontmatter.page_type, PageType::Gotcha);
    }

    #[test]
    fn frontmatter_tolerates_comments_quotes_and_block_lists() {
        // The schema block in AGENTS.md is written with inline comments, so a
        // page copied from it must still read.
        let p = parse_page(&page(
            "type: decision            # project | decision | open\n\
             project: 'baton'\n\
             status: superseded\n\
             updated: 2026-08-22       # ISO date\n\
             sources:\n  - 7123f71b\n  - \"9f2c0a11\"\n",
            "# Title\n",
        ));
        assert_eq!(p.frontmatter.page_type, PageType::Decision);
        assert_eq!(p.frontmatter.project.as_deref(), Some("baton"));
        assert_eq!(p.frontmatter.status, Status::Superseded);
        assert_eq!(p.frontmatter.sources, ["7123f71b", "9f2c0a11"]);
    }

    #[test]
    fn empty_source_list_is_allowed_but_a_missing_field_is_not() {
        let p = parse_page(&page(
            "type: open\nproject: baton\nstatus: stale\nupdated: 2026-08-22\nsources: []",
            "# Title\n",
        ));
        assert!(p.frontmatter.sources.is_empty());

        let missing = parse(
            &root(),
            &root().join("a.md"),
            &page("type: open\nproject: baton\nstatus: stale\nsources: []", "# T"),
        );
        assert!(matches!(
            missing,
            Err(WikiError::MissingField { key, .. }) if key == "updated"
        ));
    }

    #[test]
    fn an_unknown_type_or_status_names_the_field_and_the_value() {
        let bad_type = parse(
            &root(),
            &root().join("a.md"),
            &page(
                "type: note\nproject: baton\nstatus: current\nupdated: 2026-08-22\nsources: []",
                "# T",
            ),
        );
        let Err(WikiError::BadField { key, detail, .. }) = bad_type else {
            panic!("an off-schema type must not parse");
        };
        assert_eq!(key, "type");
        assert!(detail.contains("note") && detail.contains("gotcha"));

        let bad_date = parse(
            &root(),
            &root().join("a.md"),
            &page(
                "type: open\nproject: baton\nstatus: current\nupdated: yesterday\nsources: []",
                "# T",
            ),
        );
        assert!(matches!(
            bad_date,
            Err(WikiError::BadField { key, .. }) if key == "updated"
        ));
    }

    #[test]
    fn a_page_without_frontmatter_is_an_error_not_an_empty_default() {
        // index.md and log.md look like this. The walk skips them, and a direct
        // read must still refuse rather than invent a type and a date.
        assert!(matches!(
            parse(&root(), &root().join("index.md"), "# Index\n\n- [[a]]\n"),
            Err(WikiError::MissingFrontmatter { .. })
        ));
        assert!(matches!(
            parse(&root(), &root().join("a.md"), "---\ntype: open\n"),
            Err(WikiError::UnterminatedFrontmatter { .. })
        ));
    }

    #[test]
    fn crlf_and_a_byte_order_mark_parse_the_same_as_lf() {
        let text = format!("\u{feff}{}", page(FM, "# Title\n\n## Gotchas\n\nOne line.\n"))
            .replace('\n', "\r\n");
        let p = parse_page(&text);
        assert_eq!(p.title, "Title");
        assert_eq!(p.section("Gotchas").unwrap().body, "One line.");
    }

    #[test]
    fn headings_inside_a_fence_do_not_open_a_section() {
        let p = parse_page(&page(
            FM,
            "# Title\n\n\
             ## Gotchas\n\n\
             ````markdown\n## Goal\n## Current state\n````\n\n\
             Still gotchas.\n\n\
             ~~~\n## Not a heading either\n~~~\n\n\
             ## Related files\n\nsrc-tauri/src/db.rs\n",
        ));
        let headings: Vec<&str> = p.sections.iter().map(|s| s.heading.as_str()).collect();
        assert_eq!(headings, ["Gotchas", "Related files"]);
        assert!(p.section("Gotchas").unwrap().body.contains("## Goal"));
    }

    #[test]
    fn deeper_headings_stay_inside_their_section() {
        let p = parse_page(&page(
            FM,
            "# Title\n\n## Options\n\n### First\n\nCost.\n\n### Second\n\nCost.\n",
        ));
        assert_eq!(p.sections.len(), 1);
        assert!(p.section("Options").unwrap().body.starts_with("### First"));
    }

    #[test]
    fn text_before_the_first_section_is_kept_as_the_preamble() {
        let p = parse_page(&page(FM, "# Title\n\nAn intro a human added.\n\n## Gotchas\n\nOne.\n"));
        assert_eq!(p.preamble, "An intro a human added.");
        assert_eq!(p.sections.len(), 1);
    }

    #[test]
    fn links_resolve_to_paths_under_the_root() {
        let p = parse_page(&page(
            FM,
            "# Title\n\n## Gotchas\n\nSee [[concepts/mutex-across-await]] and\n\
             [[projects/baton/decisions/files-are-truth]].\n",
        ));
        assert_eq!(p.links.len(), 2);
        assert_eq!(p.links[0].target, "concepts/mutex-across-await");
        assert_eq!(
            p.links[0].path.as_deref(),
            Some(Path::new("/wiki/concepts/mutex-across-await.md"))
        );
        assert!(p.links[0].alias.is_none());
    }

    #[test]
    fn links_carry_obsidian_aliases_anchors_and_a_written_out_extension() {
        let links = links_in(
            &root(),
            "[[concepts/tauri-nspanel|the panel note]] [[concepts/pnpm-allowbuilds#The fix]] \
             [[concepts/cargo-not-on-path.md]]",
        );
        let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(
            targets,
            [
                "concepts/tauri-nspanel",
                "concepts/pnpm-allowbuilds",
                "concepts/cargo-not-on-path"
            ]
        );
        assert_eq!(links[0].alias.as_deref(), Some("the panel note"));
        assert_eq!(
            links[2].path.as_deref(),
            Some(Path::new("/wiki/concepts/cargo-not-on-path.md"))
        );
    }

    #[test]
    fn a_link_that_escapes_the_root_resolves_to_no_path() {
        let links = links_in(&root(), "[[../../.ssh/id_rsa]] and [[projects/../../x]]");
        assert_eq!(links.len(), 2);
        assert!(links.iter().all(|l| l.path.is_none()));
        // The target is still reported, so a lint can name what was written.
        assert_eq!(links[0].target, "../../.ssh/id_rsa");

        // A leading slash means the wiki root, not the filesystem root, so this
        // one is clamped rather than refused.
        let rooted = links_in(&root(), "[[/concepts/tauri-nspanel]]");
        assert_eq!(rooted[0].target, "concepts/tauri-nspanel");
        assert_eq!(
            rooted[0].path.as_deref(),
            Some(Path::new("/wiki/concepts/tauri-nspanel.md"))
        );
    }

    #[test]
    fn links_inside_code_are_examples_not_edges() {
        let p = parse_page(&page(
            FM,
            "# Title\n\n## Gotchas\n\n\
             Write `[[path/to/page]]` without the .md.\n\n\
             ```markdown\nSee [[projects/baton/overview]].\n```\n\n\
             But [[concepts/mutex-across-await]] counts.\n",
        ));
        let targets: Vec<&str> = p.links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(targets, ["concepts/mutex-across-await"]);
    }

    #[test]
    fn an_unclosed_link_does_not_swallow_the_rest_of_the_line() {
        let links = links_in(&root(), "[[broken and then [[concepts/tauri-nspanel]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "concepts/tauri-nspanel");
    }

    #[test]
    fn page_ids_are_forward_slashed_and_refuse_paths_outside_the_root() {
        let id = page_id(&root(), &root().join("projects/baton/decisions/files-are-truth.md"));
        assert_eq!(id.unwrap(), "projects/baton/decisions/files-are-truth");
        assert!(matches!(
            page_id(&root(), Path::new("/elsewhere/a.md")),
            Err(WikiError::OutsideRoot { .. })
        ));
    }

    #[test]
    fn walk_finds_pages_and_skips_the_files_that_are_not_pages() {
        let dir = std::env::temp_dir().join(format!("baton-wiki-{}", uuid::Uuid::new_v4()));
        let write = |rel: &str, body: &str| {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        };

        write("AGENTS.md", "# schema");
        write("index.md", "# Index");
        write("log.md", "# Log");
        write(".acceptance-test.md", "# hidden");
        write(".git/config", "[core]");
        write("notes.txt", "not markdown");
        write("concepts/mutex-across-await.md", &page(FM, "# A"));
        write("projects/baton/overview.md", &page(FM, "# B"));

        let found = walk(&dir).unwrap();
        let ids: Vec<String> = found.iter().map(|p| page_id(&dir, p).unwrap()).collect();
        assert_eq!(ids, ["concepts/mutex-across-await", "projects/baton/overview"]);
        assert_eq!(read(&dir, &found[1]).unwrap().title, "B");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
