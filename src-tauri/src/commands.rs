//! Tauri commands — the entire surface the webview can reach.

use tauri::{Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::context::{Context, ContextBody, ContextSummary};
use crate::db::{self, Db, Result};

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
