//! Tauri commands — the entire surface the webview can reach.

use tauri::{Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::db::{self, Db, Result};
use crate::primer;
use crate::wiki;

/// Run a closure with the index connection held.
fn with_conn<T>(db: &Db, f: impl FnOnce(&rusqlite::Connection) -> Result<T>) -> Result<T> {
    // A poisoned lock means another command panicked mid-write. Recover the
    // guard rather than cascading the panic through every later command.
    let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
    f(&conn)
}

#[tauri::command]
pub fn hide_launcher(app: tauri::AppHandle) {
    crate::launcher::hide(&app);
}

#[tauri::command]
pub fn open_main_window(app: tauri::AppHandle) -> std::result::Result<(), String> {
    if let Some(win) = app.get_webview_window(crate::BROWSER_WINDOW) {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app,
        crate::BROWSER_WINDOW,
        tauri::WebviewUrl::App("index.html?view=browser".into()),
    )
    .title("Baton")
    .inner_size(900.0, 600.0)
    .min_inner_size(640.0, 420.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------- the wiki
//
// The markdown files under ~/Baton are the source of truth. These commands read
// them and the index over them. None of them writes a page: pages are written
// by the agent that did the work, through the closing command.

/// Bring the index in line with the files. Cheap to call repeatedly, because
/// the sweep skips every file whose mtime and size match the indexed row.
#[tauri::command]
pub fn sync_wiki(app: tauri::AppHandle, db: State<'_, Db>) -> Result<db::IndexReport> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let _ = crate::onboarding::ensure_wiki(&root);
    let report = db::sync(&db, &root)?;
    // Every other reindex path rewrites index.md; this one is invoked on every
    // summon and by Refresh, so leaving it out let the catalogue drift behind
    // the tree — and the skill now trusts Baton to keep it current.
    if let Err(e) = crate::index_md::regenerate(&root) {
        eprintln!("[baton] could not rewrite index.md: {e}");
    }
    Ok(report)
}

/// Drop the index and rebuild it from the files.
///
/// Not a delete: the markdown under `~/Baton` is the source of truth and is
/// never touched. This exists for the case where the index itself is suspect.
#[tauri::command]
pub fn rebuild_index(app: tauri::AppHandle, db: State<'_, Db>) -> Result<db::IndexReport> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let _ = crate::onboarding::ensure_wiki(&root);
    with_conn(&db, db::delete_all)?;
    let report = db::sync(&db, &root)?;
    if let Err(e) = crate::index_md::regenerate(&root) {
        eprintln!("[baton] could not rewrite index.md: {e}");
    }
    Ok(report)
}

/// Projects for the launcher. One row per project, never per page.
#[tauri::command]
pub fn list_projects(db: State<'_, Db>) -> Result<Vec<db::ProjectHit>> {
    with_conn(&db, db::list_projects)
}

/// Projects matching a project name or a page title.
#[tauri::command]
pub fn search_projects(db: State<'_, Db>, query: String) -> Result<Vec<db::ProjectHit>> {
    with_conn(&db, |c| db::search_projects(c, &query))
}

#[tauri::command]
pub fn list_pages(db: State<'_, Db>) -> Result<Vec<db::PageHit>> {
    with_conn(&db, db::list_pages)
}

#[tauri::command]
pub fn search_pages(db: State<'_, Db>, query: String) -> Result<Vec<db::PageHit>> {
    with_conn(&db, |c| db::search_pages(c, &query))
}

/// Pages that link to this one.
#[tauri::command]
pub fn page_backlinks(db: State<'_, Db>, id: String) -> Result<Vec<db::PageHit>> {
    with_conn(&db, |c| db::backlinks(c, &id))
}

/// One page, read from the file rather than from the index. The index is for
/// finding a page, the file is what a reader gets, so the two cannot disagree.
#[tauri::command]
pub fn read_page(app: tauri::AppHandle, id: String) -> Result<wiki::Page> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    Ok(wiki::read(&root, &page_file(&root, &id)?)?)
}

/// Put a page on the clipboard and return it. The frontmatter is left behind,
/// because it is bookkeeping for the wiki rather than context for a model.
///
/// This is the launcher's decisive action and the reason the app exists.
#[tauri::command]
pub fn copy_page(app: tauri::AppHandle, id: String) -> Result<String> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let markdown = wiki::read(&root, &page_file(&root, &id)?)?.body;
    app.clipboard()
        .write_text(markdown.clone())
        .map_err(|e| db::DbError::Clipboard(e.to_string()))?;
    Ok(markdown)
}

/// Resolve a page id that arrived from the webview. Never join one onto the
/// root without this: `page_path` refuses anything that climbs out.
fn page_file(root: &std::path::Path, id: &str) -> Result<std::path::PathBuf> {
    wiki::page_path(root, id).ok_or_else(|| db::DbError::NotFound(id.to_string()))
}

/// Roughly what fits in a paste without crowding out the conversation that
/// follows it. Not yet configurable, and it should be once a wiki is big enough
/// for the budget to bite.
const PRIMER_BUDGET_TOKENS: usize = 12_000;

/// Compose the brief. Not a command: the launcher copies rather than previews,
/// so `copy_primer` is the only caller.
fn build_primer(app: tauri::AppHandle, project: Option<String>) -> Result<primer::Primer> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;

    // A page that will not parse is left out rather than failing the brief.
    // The sweep is what reports it, and a broken page must not cost the user
    // every other page.
    let pages: Vec<wiki::Page> = wiki::walk(&root)?
        .iter()
        .filter_map(|path| wiki::read(&root, path).ok())
        .collect();

    let project = project
        .filter(|p| !p.trim().is_empty())
        .or_else(|| primer::most_recent_project(&pages))
        .ok_or_else(|| db::DbError::NotFound("any project in the wiki".to_string()))?;

    // Lint runs over the whole wiki, not just this project: a gotcha page in
    // concepts/ can be carried into any brief.
    let lint = crate::lint::check(&pages, crate::lint::indexed_ids(&root).as_ref());

    Ok(primer::assemble(
        &pages,
        &project,
        PRIMER_BUDGET_TOKENS,
        chrono::Utc::now().date_naive(),
        &lint,
    ))
}

/// The launcher's primary action: the whole project brief on the clipboard.
#[tauri::command]
pub fn copy_primer(app: tauri::AppHandle, project: Option<String>) -> Result<primer::Primer> {
    let primer = build_primer(app.clone(), project)?;
    app.clipboard()
        .write_text(primer.text.clone())
        .map_err(|e| db::DbError::Clipboard(e.to_string()))?;
    Ok(primer)
}


// ------------------------------------------------------- first-run setup

/// Whether the wiki and the `/baton` skill exist yet. Drives the setup screen.
#[tauri::command]
pub fn wiki_status(app: tauri::AppHandle) -> Result<crate::onboarding::WikiStatus> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let home = app
        .path()
        .home_dir()
        .map_err(|e| db::DbError::Path(e.to_string()))?;
    Ok(crate::onboarding::status(&root, &home))
}

/// Write the `/baton` skill into every detected agent tool.
///
/// Explicit action, never automatic: this writes into another tool's config
/// directory, which is not ours to change on the user's behalf.
#[tauri::command]
pub fn install_skills(app: tauri::AppHandle) -> Result<Vec<String>> {
    let home = app
        .path()
        .home_dir()
        .map_err(|e| db::DbError::Path(e.to_string()))?;
    crate::onboarding::install_skills(&home).map_err(|e| db::DbError::Path(e.to_string()))
}

/// Open the wiki folder in the file manager, so "where do my notes live?" has
/// an answer that does not involve typing a path.
#[tauri::command]
pub fn reveal_wiki(app: tauri::AppHandle) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    app.opener()
        .open_path(root.display().to_string(), None::<&str>)
        .map_err(|e| db::DbError::Path(e.to_string()))
}
