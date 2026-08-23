//! Watches the wiki folder and reindexes when it changes.
//!
//! Without this, the index is only built at startup — so pages the agent writes
//! during a session stay invisible until the app restarts. That is the single
//! most obvious way the launcher can lie about what you know.
//!
//! Debounced because editors and agents write more than once per logical save:
//! a temp file, a rename, a metadata touch. Reindexing on every raw event would
//! reparse the whole wiki several times per keystroke.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use tauri::{AppHandle, Emitter, Manager};

use crate::db::{self, Db};

/// Emitted after a reindex so open windows can refresh.
pub const CHANGED_EVENT: &str = "wiki-changed";

/// Long enough to collapse an editor's write-rename-touch burst, short enough
/// that a page written by an agent shows up while the user is still looking.
const DEBOUNCE: Duration = Duration::from_millis(400);

/// True for paths that should trigger a reindex.
///
/// Editors litter the tree with swap files, and `.git` churns on every command
/// — reindexing on those would be constant and pointless.
fn is_relevant(path: &Path) -> bool {
    if path
        .components()
        .any(|c| c.as_os_str() == ".git" || c.as_os_str() == ".DS_Store")
    {
        return false;
    }
    // AGENTS.md is the schema, index.md is derived from the tree and log.md is
    // an append-only journal. None is a page. Reacting to index.md in
    // particular would mean every reindex triggered another one.
    if matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md") | Some("log.md") | Some("AGENTS.md")
    ) {
        return false;
    }
    match path.extension().and_then(|e| e.to_str()) {
        // A deletion arrives with no extension left to inspect, so anything
        // extensionless is treated as relevant rather than silently ignored.
        None => true,
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            // Vim writes `.md~` and `.md.swp`; VS Code writes `.md.tmp`.
            ext == "md" && !path.to_string_lossy().ends_with('~')
        }
    }
}

fn reindex(app: &AppHandle, root: &Path) {
    let db = app.state::<Db>();
    match db::sync(&db, root) {
        Ok(report) => {
            for error in &report.errors {
                eprintln!("[baton] wiki page skipped: {error}");
            }
            // The skill reads index.md to decide whether a subject already has
            // a page, so a stale index causes the duplicate pages the schema
            // exists to prevent.
            if let Err(e) = crate::index_md::regenerate(root) {
                eprintln!("[baton] could not rewrite index.md: {e}");
            }
            // Windows refresh themselves rather than polling; a wiki that
            // changed but looks unchanged is the bug this whole module exists
            // to prevent.
            let _ = app.emit(CHANGED_EVENT, report);
        }
        Err(e) => eprintln!("[baton] reindex failed: {e}"),
    }
}

/// Start watching. The returned guard must be kept alive for the app's
/// lifetime — dropping it stops the watcher silently.
pub fn spawn(app: &AppHandle, root: PathBuf) {
    let handle = app.clone();
    let watch_root = root.clone();

    // The debouncer owns a thread and must outlive this function, so it is
    // parked in Tauri's state rather than dropped at the end of the scope.
    let result = new_debouncer(
        DEBOUNCE,
        None,
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                if events.iter().any(|e| e.paths.iter().any(|p| is_relevant(p))) {
                    reindex(&handle, &watch_root);
                }
            }
            // A watch error is not fatal: the startup index is still valid and
            // the user can reindex by hand.
            Err(errors) => {
                for e in errors {
                    eprintln!("[baton] watch error: {e}");
                }
            }
        },
    );

    match result {
        Ok(mut debouncer) => {
            if let Err(e) = debouncer.watch(&root, RecursiveMode::Recursive) {
                eprintln!("[baton] could not watch {}: {e}", root.display());
                return;
            }
            app.manage(WatcherGuard(debouncer));
        }
        Err(e) => eprintln!("[baton] could not start watcher: {e}"),
    }
}

/// Holds the debouncer alive. The field is never read on purpose: dropping the
/// debouncer stops the watcher, so its only job is to exist for as long as the
/// app does.
#[allow(dead_code)]
struct WatcherGuard(
    notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::RecommendedCache,
    >,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_markdown_triggers_a_reindex() {
        assert!(is_relevant(Path::new("/w/projects/a/overview.md")));
        assert!(is_relevant(Path::new("/w/AGENTS.MD")), "extension case");
    }

    #[test]
    fn editor_and_vcs_noise_is_ignored() {
        // Reindexing on these would be near-constant while editing.
        for p in [
            "/w/.git/index",
            "/w/.git/refs/heads/main",
            "/w/projects/a/.DS_Store",
            "/w/projects/a/overview.md~",
            "/w/projects/a/overview.md.swp",
            "/w/notes.txt",
        ] {
            assert!(!is_relevant(Path::new(p)), "{p} should be ignored");
        }
    }

    #[test]
    fn derived_files_do_not_trigger_a_reindex() {
        // index.md is written *by* the reindex; reacting to it would loop.
        for p in ["/w/index.md", "/w/log.md", "/w/AGENTS.md"] {
            assert!(!is_relevant(Path::new(p)), "{p} should be ignored");
        }
    }

    #[test]
    fn extensionless_paths_stay_relevant() {
        // A directory rename or a delete can arrive without a usable
        // extension; ignoring those would drop real removals from the index.
        assert!(is_relevant(Path::new("/w/projects/a")));
    }
}
