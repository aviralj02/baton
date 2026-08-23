//! First-run setup.
//!
//! Baton is useless until two things exist that the app itself did not ship:
//! the wiki folder with its schema, and a `/baton` skill inside whichever agent
//! tool the user runs. Without them a fresh install reaches an empty launcher
//! telling the user to run a command that does not exist.
//!
//! The two halves are treated differently on purpose:
//!
//! * The **wiki folder** is Baton's own data. It is created silently on first
//!   run, the way any app creates its data directory.
//! * The **skill** is written into another tool's configuration, so it is never
//!   installed without the user asking. It is offered, not assumed.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// The schema and the command, embedded from `skills/` at the repo root.
///
/// `skills/` is the single source of truth: it is what a human copies after a
/// clone, and what the app installs. There were briefly two vendored copies of
/// each — one here, one in `skills/` — and they drifted within a day, with the
/// app shipping a schema 121 lines behind the one in the repo.
const AGENTS_MD: &str = include_str!("../../skills/wiki/AGENTS.md");
const SKILL_MD: &str = include_str!("../../skills/baton/SKILL.md");

/// Seeds for the two derived files. `index.md` is rewritten from the tree on
/// the first sweep, so this is only what an empty wiki shows.
const INDEX_MD: &str = "# Baton index\n\nEvery page, grouped by project and then by type. Derived from the tree — edit the tree, not this file.\n\n_No pages yet. Run `/baton` at the end of a working session and the agent will propose the first ones._\n";
const LOG_MD: &str = "# Log\n\nAppend-only record of what changed and when. Newest last, one entry per filing session.\n";

/// Agent tools that read a `skills/` directory. Only ones already present on
/// the machine are offered — creating `~/.codex/` for someone who does not use
/// Codex would be litter.
const SKILL_HOSTS: &[(&str, &str)] = &[
    ("Claude Code", ".claude"),
    ("Codex", ".codex"),
    ("Cursor", ".cursor"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiStatus {
    pub root: String,
    /// False on a fresh install — the trigger for showing setup.
    pub root_exists: bool,
    pub has_schema: bool,
    pub page_count: usize,
    pub skills: Vec<SkillHost>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHost {
    pub name: String,
    /// The tool's config directory exists, so it is worth offering.
    pub detected: bool,
    /// A `/baton` skill is already installed for it.
    pub installed: bool,
    /// True when installed but the contents differ from the vendored copy.
    pub outdated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupReport {
    pub created_root: bool,
    pub created_files: Vec<String>,
    pub installed_skills: Vec<String>,
}

pub type Result<T> = std::result::Result<T, std::io::Error>;

fn skill_file(home: &Path, dir: &str) -> PathBuf {
    home.join(dir).join("skills").join("baton").join("SKILL.md")
}

/// Create the wiki folder and its non-page files if they are missing.
///
/// Never overwrites: a user who has edited their schema keeps their edits, and
/// re-running this is safe. That also makes it callable unconditionally at
/// startup without a "have we done this?" flag to get out of sync.
pub fn ensure_wiki(root: &Path) -> Result<SetupReport> {
    let created_root = !root.exists();

    std::fs::create_dir_all(root.join("projects"))?;
    std::fs::create_dir_all(root.join("concepts"))?;

    let mut created_files = Vec::new();
    for (name, body) in [
        ("AGENTS.md", AGENTS_MD),
        ("index.md", INDEX_MD),
        ("log.md", LOG_MD),
    ] {
        let path = root.join(name);
        if !path.exists() {
            std::fs::write(&path, body)?;
            created_files.push(name.to_string());
        }
    }

    Ok(SetupReport {
        created_root,
        created_files,
        installed_skills: Vec::new(),
    })
}

/// Which agent tools are present, and whether each already has the skill.
pub fn skill_hosts(home: &Path) -> Vec<SkillHost> {
    SKILL_HOSTS
        .iter()
        .map(|(name, dir)| {
            let file = skill_file(home, dir);
            let installed = file.exists();
            // A skill from an older Baton is worse than none: it may describe a
            // schema the parser no longer accepts, and nothing else would ever
            // tell the user it drifted.
            let outdated = installed
                && std::fs::read_to_string(&file)
                    .map(|current| current != SKILL_MD)
                    .unwrap_or(false);
            SkillHost {
                name: (*name).to_string(),
                detected: home.join(dir).is_dir(),
                installed,
                outdated,
            }
        })
        .collect()
}

/// Write the skill into every detected tool. Explicit action only.
pub fn install_skills(home: &Path) -> Result<Vec<String>> {
    let mut installed = Vec::new();
    for (name, dir) in SKILL_HOSTS {
        if !home.join(dir).is_dir() {
            continue;
        }
        let file = skill_file(home, dir);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, SKILL_MD)?;
        installed.push((*name).to_string());
    }
    Ok(installed)
}

/// Count real pages. `AGENTS.md`, `index.md` and `log.md` carry no frontmatter
/// and are not pages, so they must not make an empty wiki look populated.
pub fn page_count(root: &Path) -> usize {
    crate::wiki::walk(root).map(|paths| paths.len()).unwrap_or(0)
}

pub fn status(root: &Path, home: &Path) -> WikiStatus {
    WikiStatus {
        root: root.display().to_string(),
        root_exists: root.is_dir(),
        has_schema: root.join("AGENTS.md").is_file(),
        page_count: page_count(root),
        skills: skill_hosts(home),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baton-onboarding-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn ensure_wiki_creates_a_usable_wiki() {
        let root = tmp("create");
        let report = ensure_wiki(&root).unwrap();

        assert!(report.created_root);
        assert!(root.join("projects").is_dir());
        assert!(root.join("concepts").is_dir());
        for f in ["AGENTS.md", "index.md", "log.md"] {
            assert!(root.join(f).is_file(), "{f} missing");
            assert!(report.created_files.contains(&f.to_string()));
        }
        // The schema must be the real one, not a stub — the agent writes
        // against it and a placeholder would produce invalid pages.
        let schema = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert!(schema.contains("## Frontmatter"), "schema looks like a stub");
    }

    #[test]
    fn ensure_wiki_never_overwrites_user_edits() {
        let root = tmp("idempotent");
        ensure_wiki(&root).unwrap();
        std::fs::write(root.join("AGENTS.md"), "# my own schema").unwrap();

        let second = ensure_wiki(&root).unwrap();
        assert!(!second.created_root);
        assert!(second.created_files.is_empty(), "re-created existing files");
        assert_eq!(
            std::fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "# my own schema",
            "clobbered a hand-edited schema"
        );
    }

    #[test]
    fn the_three_non_pages_are_not_counted_as_pages() {
        let root = tmp("count");
        ensure_wiki(&root).unwrap();
        assert_eq!(page_count(&root), 0, "AGENTS/index/log counted as pages");
    }

    #[test]
    fn skills_install_only_into_detected_tools() {
        let home = tmp("skills");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        // .codex and .cursor deliberately absent.

        let installed = install_skills(&home).unwrap();
        assert_eq!(installed, vec!["Claude Code"]);
        assert!(skill_file(&home, ".claude").is_file());
        assert!(!home.join(".codex").exists(), "created a dir for an absent tool");
    }

    #[test]
    fn status_reports_installed_and_outdated_skills() {
        let home = tmp("status");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        install_skills(&home).unwrap();

        let claude = |hosts: Vec<SkillHost>| {
            hosts.into_iter().find(|h| h.name == "Claude Code").unwrap()
        };

        let fresh = claude(skill_hosts(&home));
        assert!(fresh.detected && fresh.installed && !fresh.outdated);

        // A stale copy from an older Baton must announce itself.
        std::fs::write(skill_file(&home, ".claude"), "# old version").unwrap();
        assert!(claude(skill_hosts(&home)).outdated);
    }
}
