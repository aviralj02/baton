//! Tauri commands — the entire surface the webview can reach.

use tauri::{Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::ai::{self, AiClient};
use crate::context::{Context, ContextBody, ContextSummary};
use crate::db::{self, Db, Result};
use crate::primer;
use crate::wiki;

fn with_conn<T>(db: &Db, f: impl FnOnce(&rusqlite::Connection) -> Result<T>) -> Result<T> {
    // A poisoned lock means another command panicked mid-write. Recover the
    // guard rather than cascading the panic through every later command.
    let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
    f(&conn)
}

#[tauri::command]
pub fn list_contexts(db: State<'_, Db>) -> Result<Vec<ContextSummary>> {
    with_conn(&db, db::list)
}

#[tauri::command]
pub fn search_contexts(db: State<'_, Db>, query: String) -> Result<Vec<ContextSummary>> {
    with_conn(&db, |c| db::search(c, &query))
}

#[tauri::command]
pub fn get_context(db: State<'_, Db>, id: String) -> Result<Context> {
    with_conn(&db, |c| db::get(c, &id))
}

/// Create or update. An empty `id` means "new" — the id is minted here rather
/// than in the webview so it can never collide or be forged.
#[tauri::command]
pub fn save_context(
    db: State<'_, Db>,
    id: Option<String>,
    name: String,
    body: Option<ContextBody>,
) -> Result<Context> {
    let ctx = Context {
        id: id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name,
        body: body.unwrap_or_default(),
        created_at: String::new(), // set by the DB layer
        updated_at: String::new(),
    };
    with_conn(&db, |c| db::save(c, &ctx))
}

#[tauri::command]
pub fn delete_context(db: State<'_, Db>, id: String) -> Result<()> {
    with_conn(&db, |c| db::delete(c, &id))
}

/// PRD §9: a single explicit action that removes everything.
#[tauri::command]
pub fn delete_all_data(db: State<'_, Db>) -> Result<()> {
    with_conn(&db, db::delete_all)
}

#[tauri::command]
pub fn copy_context(app: tauri::AppHandle, db: State<'_, Db>, id: String) -> Result<String> {
    let markdown = with_conn(&db, |c| db::get(c, &id))?.to_markdown();
    app.clipboard()
        .write_text(markdown.clone())
        .map_err(|e| db::DbError::Clipboard(e.to_string()))?;
    Ok(markdown)
}

/// Preview without touching the clipboard — used by the detail view.
#[tauri::command]
pub fn render_context(db: State<'_, Db>, id: String) -> Result<String> {
    Ok(with_conn(&db, |c| db::get(c, &id))?.to_markdown())
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
    let report = db::sync(&db, &root)?;
    // Every reindex path rewrites index.md. Leaving it out of this one let the
    // catalogue drift behind the tree, and the skill trusts Baton to keep it
    // current.
    if let Err(e) = crate::index_md::regenerate(&root) {
        eprintln!("[baton] could not rewrite index.md: {e}");
    }
    Ok(report)
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

/// Links that name a page which does not exist. The wiki's own lint.
#[tauri::command]
pub fn broken_links(db: State<'_, Db>) -> Result<Vec<db::BrokenLink>> {
    with_conn(&db, db::broken_links)
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

/// Compose the brief without copying it, so the launcher can show the estimate
/// before the user commits to a paste. Reading and parsing the whole wiki takes
/// well under a millisecond, so this runs on every summon.
#[tauri::command]
pub fn build_primer(app: tauri::AppHandle, project: Option<String>) -> Result<primer::Primer> {
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

    Ok(primer::assemble(
        &pages,
        &project,
        PRIMER_BUDGET_TOKENS,
        chrono::Utc::now().date_naive(),
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

// ------------------------------------------------------------------ AI
//
// These are the only commands that send anything off the machine, and they run
// only when the user explicitly asks (PRD §9). The lock is taken, released,
// then re-taken around the request — holding a std Mutex across an await would
// block every other command for the whole call.

/// Generate a context from a pasted conversation and save it.
#[tauri::command]
pub async fn create_context_from_conversation(
    app: tauri::AppHandle,
    name: String,
    conversation: String,
) -> std::result::Result<Context, String> {
    let body = AiClient::new(crate::device_id(&app))
        .extract(&conversation)
        .await
        .map_err(|e| e.to_string())?;

    let db = app.state::<Db>();
    let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = Context {
        id: uuid::Uuid::new_v4().to_string(),
        name: pick_name(name, &body),
        body,
        created_at: String::new(),
        updated_at: String::new(),
    };
    let saved = db::save(&conn, &ctx).map_err(|e| e.to_string())?;
    db::add_source(&conn, &saved.id, "conversation", &conversation)
        .map_err(|e| e.to_string())?;
    Ok(saved)
}

/// Merge a newer conversation into an existing context (PRD §9 update rules).
#[tauri::command]
pub async fn update_context_from_conversation(
    app: tauri::AppHandle,
    id: String,
    conversation: String,
) -> std::result::Result<Context, String> {
    let existing = {
        let db = app.state::<Db>();
        let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
        db::get(&conn, &id).map_err(|e| e.to_string())?
    };

    let body = AiClient::new(crate::device_id(&app))
        .update(&existing, &conversation)
        .await
        .map_err(|e| e.to_string())?;

    let db = app.state::<Db>();
    let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
    let merged = Context { body, ..existing };
    let saved = db::save(&conn, &merged).map_err(|e| e.to_string())?;
    db::add_source(&conn, &saved.id, "conversation", &conversation)
        .map_err(|e| e.to_string())?;
    Ok(saved)
}

/// Render a continuation prompt aimed at the next model, and copy it.
#[tauri::command]
pub async fn generate_handoff(
    app: tauri::AppHandle,
    id: String,
) -> std::result::Result<String, String> {
    let ctx = {
        let db = app.state::<Db>();
        let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
        db::get(&conn, &id).map_err(|e| e.to_string())?
    };

    let prompt = AiClient::new(crate::device_id(&app))
        .handoff(&ctx)
        .await
        .map_err(|e| e.to_string())?;

    app.clipboard()
        .write_text(prompt.clone())
        .map_err(|e| e.to_string())?;
    Ok(prompt)
}

/// Whether generation is available — the UI hides AI actions when it is not.
#[tauri::command]
pub fn ai_endpoint() -> String {
    ai::base_url().to_string()
}

/// Fall back to the model's own description when the user did not name it,
/// so a context created straight from a paste is never called "Untitled".
fn pick_name(name: String, body: &ContextBody) -> String {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    body.description
        .as_deref()
        .or(body.goal.as_deref())
        .map(|s| {
            let first: String = s.trim().chars().take(60).collect();
            first.split(['.', '\n']).next().unwrap_or(&first).trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled context".to_string())
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
