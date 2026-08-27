//! Deleting pages, projects and the whole wiki.
//!
//! This is the one place Baton touches a user's markdown, and it is a deliberate
//! exception to "Baton does not write pages" rather than a hole in it. The rule
//! exists so there is a single path that can *author* a page — a second author
//! is a second place for the schema to be violated. Removal cannot violate a
//! schema, and a knowledge base you can only add to is a hoard.
//!
//! Everything here goes to the OS trash rather than being unlinked. The wiki is
//! months of accumulated context, the buttons that call this sit next to
//! everyday ones, and "the files are yours" is the whole product promise — a
//! permanent delete behind one confirm would contradict it. Recovery is then the
//! Finder or the Recycle Bin, which the user already knows how to use.
//!
//! Nothing here touches the index. Removing the files is the whole operation;
//! the caller re-syncs, and `db::sync` drops the rows whose files have gone. An
//! index that deleted its own rows would disagree with the tree the moment a
//! trash operation half-failed.

use std::path::{Path, PathBuf};

use crate::db::{DbError, Result};

/// The file behind a page id, verified to exist and to sit inside the root.
///
/// `wiki::page_path` is what refuses an id that climbs out with `..`; without
/// it a crafted id from the webview would trash an arbitrary file.
pub fn page(root: &Path, id: &str) -> Result<PathBuf> {
    let path = crate::wiki::page_path(root, id).ok_or_else(|| DbError::NotFound(id.to_string()))?;
    if !path.is_file() {
        return Err(DbError::NotFound(id.to_string()));
    }
    Ok(path)
}

/// The directory holding one project's pages.
///
/// The slug is checked the same way a page id is: it arrives from the webview,
/// and `projects/../..` would otherwise resolve to the home directory.
pub fn project(root: &Path, slug: &str) -> Result<PathBuf> {
    let dir = crate::wiki::page_path(root, &format!("projects/{slug}"))
        .ok_or_else(|| DbError::NotFound(slug.to_string()))?
        // page_path appends .md, because every other caller wants a file.
        .with_extension("");
    if !dir.is_dir() {
        return Err(DbError::NotFound(slug.to_string()));
    }
    Ok(dir)
}

/// Everything that holds pages: the project folders and the constraints.
///
/// `AGENTS.md` survives deliberately. It is the schema the user may have edited,
/// it is not a page, and `ensure_wiki` would only write a fresh copy back — so
/// removing it costs the user their edits and changes nothing else. `index.md`
/// and `log.md` are derived and regenerate on the next sweep.
pub fn everything(root: &Path) -> Result<Vec<PathBuf>> {
    let mut targets = Vec::new();
    for name in ["projects", "concepts"] {
        let dir = root.join(name);
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir).map_err(|e| DbError::Path(e.to_string()))? {
            targets.push(entry.map_err(|e| DbError::Path(e.to_string()))?.path());
        }
    }
    Ok(targets)
}

/// Move paths to the OS trash. A no-op on an empty list, so "delete all" on an
/// empty wiki succeeds quietly rather than erroring.
pub fn discard(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    trash::delete_all(paths).map_err(|e| DbError::Path(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wiki(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("baton-remove-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("projects/demo/decisions")).unwrap();
        std::fs::create_dir_all(root.join("concepts")).unwrap();
        std::fs::write(root.join("AGENTS.md"), "# schema").unwrap();
        std::fs::write(root.join("projects/demo/overview.md"), "# demo").unwrap();
        std::fs::write(root.join("projects/demo/decisions/why.md"), "# why").unwrap();
        std::fs::write(root.join("concepts/appkit.md"), "# appkit").unwrap();
        root
    }

    #[test]
    fn resolves_a_page_to_its_file() {
        let root = wiki("page");
        assert_eq!(
            page(&root, "projects/demo/overview").unwrap(),
            root.join("projects/demo/overview.md")
        );
    }

    #[test]
    fn a_page_that_is_not_there_is_not_found() {
        let root = wiki("missing");
        assert!(page(&root, "projects/demo/ghost").is_err());
    }

    #[test]
    fn an_id_that_climbs_out_of_the_root_is_refused() {
        let root = wiki("escape");
        // The file exists, so only the traversal guard can reject this.
        std::fs::write(root.parent().unwrap().join("outside.md"), "x").unwrap();
        assert!(page(&root, "../outside").is_err());
        assert!(page(&root, "/etc/passwd").is_err());
        assert!(project(&root, "../..").is_err());
    }

    #[test]
    fn resolves_a_project_to_its_directory() {
        let root = wiki("project");
        assert_eq!(
            project(&root, "demo").unwrap(),
            root.join("projects/demo"),
            "a project is the folder, not the overview page"
        );
    }

    #[test]
    fn everything_lists_projects_and_constraints_but_spares_the_schema() {
        let root = wiki("all");
        let targets = everything(&root).unwrap();

        assert!(targets.contains(&root.join("projects/demo")));
        assert!(targets.contains(&root.join("concepts/appkit.md")));
        assert!(
            !targets.contains(&root.join("AGENTS.md")),
            "the schema is not a page and may carry the user's edits"
        );
    }

    /// Ignored because it puts real items in the real Trash. Run deliberately
    /// with `cargo test -- --ignored` after touching this module or bumping the
    /// `trash` crate: it is the only check that the platform call works at all,
    /// and that it accepts a directory as well as a file.
    #[test]
    #[ignore]
    fn discard_really_trashes_a_file_and_a_directory() {
        let root = wiki("discard");
        let file = root.join("concepts/appkit.md");
        let dir = root.join("projects/demo");

        discard(&[file.clone(), dir.clone()]).unwrap();

        assert!(!file.exists(), "file survived");
        assert!(!dir.exists(), "directory survived");
        assert!(root.join("AGENTS.md").exists(), "took more than it was given");
    }

    #[test]
    fn everything_on_an_empty_wiki_is_empty_rather_than_an_error() {
        let root = std::env::temp_dir().join("baton-remove-bare");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        assert!(everything(&root).unwrap().is_empty());
        assert!(discard(&[]).is_ok());
    }
}
