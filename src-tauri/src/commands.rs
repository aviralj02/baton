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

// ------------------------------------------------------------- deleting
//
// The one place Baton touches the user's markdown. Everything goes to the OS
// trash; see `remove.rs` for why that is not merely politeness.

/// Trash `paths`, then bring the index back in line with what is left.
///
/// The re-sync is what removes the rows, not a delete statement: `db::sync`
/// drops every page whose file has gone, so a trash operation that only half
/// succeeded leaves an index that still matches the tree.
fn discard_and_resync(
    app: &tauri::AppHandle,
    db: &Db,
    paths: Vec<std::path::PathBuf>,
) -> Result<db::IndexReport> {
    let root = crate::wiki_root(app).map_err(db::DbError::Path)?;
    crate::remove::discard(&paths)?;
    let report = db::sync(db, &root)?;
    if let Err(e) = crate::index_md::regenerate(&root) {
        eprintln!("[baton] could not rewrite index.md: {e}");
    }
    Ok(report)
}

/// Move one page to the trash.
#[tauri::command]
pub fn delete_page(app: tauri::AppHandle, db: State<'_, Db>, id: String) -> Result<db::IndexReport> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let path = crate::remove::page(&root, &id)?;
    discard_and_resync(&app, &db, vec![path])
}

/// Move a whole project — every page under `projects/<slug>/` — to the trash.
#[tauri::command]
pub fn delete_project(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    slug: String,
) -> Result<db::IndexReport> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let dir = crate::remove::project(&root, &slug)?;
    discard_and_resync(&app, &db, vec![dir])
}

/// Move every project and constraint to the trash, leaving an empty wiki.
///
/// `AGENTS.md` stays: it is the schema, not a page, and it may carry the user's
/// own edits. What is left is what a fresh install has.
#[tauri::command]
pub fn delete_everything(app: tauri::AppHandle, db: State<'_, Db>) -> Result<db::IndexReport> {
    let root = crate::wiki_root(&app).map_err(db::DbError::Path)?;
    let targets = crate::remove::everything(&root)?;
    discard_and_resync(&app, &db, targets)
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


// ------------------------------------------------------------- settings

/// Problems recorded before a window existed to show them.
#[tauri::command]
pub fn take_notices(app: tauri::AppHandle) -> Vec<String> {
    crate::notice::take(&app)
}

/// The summon shortcut, as an accelerator string.
#[tauri::command]
pub fn get_shortcut(app: tauri::AppHandle) -> Result<String> {
    let dir = app.path().app_data_dir().map_err(|e| db::DbError::Path(e.to_string()))?;
    Ok(crate::settings::load(&dir).shortcut)
}

/// Register a new summon shortcut and persist it.
///
/// The old one is released first: leaving it registered would give the launcher
/// two hotkeys, one of which the user believes they removed. Persisted only
/// after registration succeeds, so a rejected combination cannot survive a
/// restart as a shortcut that does nothing.
#[tauri::command]
pub fn set_shortcut(app: tauri::AppHandle, accelerator: String) -> std::result::Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let next = crate::settings::parse(&accelerator)?;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active = app.state::<crate::settings::Active>();
    let previous = *active.0.lock().map_err(|e| e.to_string())?;

    if next == previous {
        return Ok(());
    }

    let _ = app.global_shortcut().unregister(previous);
    if app.global_shortcut().register(next).is_err() {
        // Put the old one back rather than leaving the user with no hotkey.
        let _ = app.global_shortcut().register(previous);
        return Err(format!("{accelerator} is already taken by another app"));
    }

    *active.0.lock().map_err(|e| e.to_string())? = next;
    crate::settings::save(&dir, &crate::settings::Settings { shortcut: accelerator })
        .map_err(|e| e.to_string())
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
