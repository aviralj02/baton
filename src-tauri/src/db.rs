//! SQLite storage. Owned entirely by Rust — the webview never touches it.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::context::{Context, ContextBody, ContextSummary};
use crate::wiki::{self, Page};

/// Managed Tauri state. `rusqlite::Connection` is `Send` but not `Sync`, so it
/// needs the mutex to live in shared state.
pub struct Db(pub Mutex<Connection>);

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored context is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no context with id {0}")]
    NotFound(String),
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("cannot locate the wiki: {0}")]
    Path(String),
    #[error(transparent)]
    Wiki(#[from] wiki::WikiError),
}

impl serde::Serialize for DbError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, DbError>;

pub fn open(path: &std::path::Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Versioned migrations via `user_version`. Each step runs once, in order.
fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE contexts (
              id         TEXT PRIMARY KEY,
              name       TEXT NOT NULL,
              content    TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE sources (
              id         TEXT PRIMARY KEY,
              context_id TEXT NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
              type       TEXT NOT NULL,
              content    TEXT NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE INDEX idx_sources_context ON sources(context_id);

            -- External-content FTS: the index stores no copy of the rows, it
            -- points at `contexts` by rowid. That means the triggers below are
            -- mandatory, not an optimisation — without them the index silently
            -- drifts out of sync with the table.
            CREATE VIRTUAL TABLE contexts_fts USING fts5(
              name, content, content='contexts', content_rowid='rowid'
            );

            CREATE TRIGGER contexts_ai AFTER INSERT ON contexts BEGIN
              INSERT INTO contexts_fts(rowid, name, content)
              VALUES (new.rowid, new.name, new.content);
            END;

            CREATE TRIGGER contexts_ad AFTER DELETE ON contexts BEGIN
              INSERT INTO contexts_fts(contexts_fts, rowid, name, content)
              VALUES ('delete', old.rowid, old.name, old.content);
            END;

            CREATE TRIGGER contexts_au AFTER UPDATE ON contexts BEGIN
              INSERT INTO contexts_fts(contexts_fts, rowid, name, content)
              VALUES ('delete', old.rowid, old.name, old.content);
              INSERT INTO contexts_fts(rowid, name, content)
              VALUES (new.rowid, new.name, new.content);
            END;

            PRAGMA user_version = 1;
            "#,
        )?;
    }

    if version < 2 {
        // The wiki index. Every row here is derived from a markdown file under
        // ~/Baton and can be deleted and rebuilt at any time.
        //
        // No triggers, unlike `contexts` above. `pages` is not the source of
        // truth, so a write is always "replace everything this page had", which
        // the indexer does in one transaction. `pages_fts` is a plain FTS5
        // table rather than external-content for the same reason: it keeps its
        // own copy, so a stale row is removed by a DELETE with a WHERE clause
        // instead of the external-content 'delete' command, which needs the old
        // values and is the part that silently drifts when it is wrong.
        conn.execute_batch(
            r#"
            CREATE TABLE pages (
              id         TEXT PRIMARY KEY,   -- 'projects/baton/db', from wiki::page_id
              path       TEXT NOT NULL,
              type       TEXT NOT NULL,
              project    TEXT,               -- NULL for pages in concepts/
              status     TEXT NOT NULL,
              updated    TEXT NOT NULL,      -- the frontmatter date, ISO
              title      TEXT NOT NULL,
              body       TEXT NOT NULL,
              mtime      INTEGER NOT NULL,   -- epoch ms, the change gate
              size       INTEGER NOT NULL,
              indexed_at TEXT NOT NULL
            );

            CREATE INDEX idx_pages_project ON pages(project);

            CREATE TABLE sections (
              page_id TEXT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
              ord     INTEGER NOT NULL,
              heading TEXT NOT NULL,
              body    TEXT NOT NULL,
              PRIMARY KEY (page_id, ord)
            );

            -- `dst` is deliberately not a foreign key: a link may name a page
            -- that does not exist, and finding those is the point.
            CREATE TABLE links (
              src TEXT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
              dst TEXT NOT NULL,
              PRIMARY KEY (src, dst)
            );

            CREATE INDEX idx_links_dst ON links(dst);

            CREATE VIRTUAL TABLE pages_fts USING fts5(id UNINDEXED, title, body);

            CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

            PRAGMA user_version = 2;
            "#,
        )?;
    }

    Ok(())
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn row_to_context(row: &rusqlite::Row) -> rusqlite::Result<(String, String, String, String, String)> {
    Ok((
        row.get("id")?,
        row.get("name")?,
        row.get("content")?,
        row.get("created_at")?,
        row.get("updated_at")?,
    ))
}

fn build(
    (id, name, content, created_at, updated_at): (String, String, String, String, String),
) -> Result<Context> {
    let body: ContextBody = serde_json::from_str(&content)?;
    Ok(Context {
        id,
        name,
        body,
        created_at,
        updated_at,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<ContextSummary>> {
    let mut stmt = conn
        .prepare("SELECT id, name, updated_at FROM contexts ORDER BY updated_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(ContextSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> Result<Context> {
    let row = conn
        .query_row(
            "SELECT id, name, content, created_at, updated_at FROM contexts WHERE id = ?1",
            params![id],
            row_to_context,
        )
        .optional()?;
    match row {
        Some(r) => build(r),
        None => Err(DbError::NotFound(id.to_string())),
    }
}

/// Insert or update. Returns the stored context so the caller gets the
/// authoritative timestamps back.
pub fn save(conn: &Connection, ctx: &Context) -> Result<Context> {
    let content = serde_json::to_string(&ctx.body)?;
    let now = now();

    let existing_created: Option<String> = conn
        .query_row(
            "SELECT created_at FROM contexts WHERE id = ?1",
            params![ctx.id],
            |r| r.get(0),
        )
        .optional()?;

    let created_at = existing_created.unwrap_or_else(|| now.clone());

    conn.execute(
        "INSERT INTO contexts (id, name, content, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
           name = excluded.name,
           content = excluded.content,
           updated_at = excluded.updated_at",
        params![ctx.id, ctx.name.trim(), content, created_at, now],
    )?;

    get(conn, &ctx.id)
}

/// Record the raw conversation an extraction came from (PRD §6 `sources`).
///
/// Kept so a context can be re-extracted with a better prompt later without
/// asking the user to go find the conversation again. Note this is the one
/// place Baton stores unfiltered chat text — see PRD §16 on encrypting or
/// expiring it, since real conversations contain credentials.
pub fn add_source(conn: &Connection, context_id: &str, kind: &str, content: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO sources (id, context_id, type, content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            uuid::Uuid::new_v4().to_string(),
            context_id,
            kind,
            content,
            now()
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM contexts WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(DbError::NotFound(id.to_string()));
    }
    Ok(())
}

/// PRD §9's "remove everything" action, at the storage layer.
///
/// This clears the local database only. The wiki files under ~/Baton are the
/// source of truth and are never touched here, so the next sweep rebuilds
/// `pages` from disk. Dropping `meta` is what makes that sweep a full one.
pub fn delete_all(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM contexts;
         DELETE FROM sources;
         DELETE FROM pages;
         DELETE FROM sections;
         DELETE FROM links;
         DELETE FROM pages_fts;
         DELETE FROM meta;",
    )?;
    Ok(())
}

fn to_fts_query(raw: &str) -> Option<String> {
    let terms: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.to_lowercase()))
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

pub fn search(conn: &Connection, raw: &str) -> Result<Vec<ContextSummary>> {
    let Some(query) = to_fts_query(raw) else {
        return list(conn);
    };

    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.updated_at
         FROM contexts_fts f
         JOIN contexts c ON c.rowid = f.rowid
         WHERE contexts_fts MATCH ?1
         ORDER BY bm25(contexts_fts, 10.0, 1.0), c.updated_at DESC",
    )?;
    let rows = stmt.query_map(params![query], |row| {
        Ok(ContextSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ----------------------------------------------------------- the wiki index
//
// Everything below is derived from the markdown files under ~/Baton. None of it
// is authoritative, and deleting the database costs nothing but a re-sweep.

/// Bump this when `wiki.rs` changes the shape of what a parse produces.
///
/// The sweep skips any file whose mtime and size match the indexed row. That
/// gate is what makes a summon-time sweep free, and it is also a trap: without
/// a version to compare, a parser change would leave every unchanged file
/// holding rows in the old shape, silently. `user_version` tracks the schema,
/// this tracks the parser.
pub const INDEXER_VERSION: i64 = 1;

const INDEXER_VERSION_KEY: &str = "indexer_version";

/// What a page looked like on disk when it was indexed. Epoch milliseconds and
/// a byte count, both wide enough in SQLite's 64-bit INTEGER.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStat {
    pub mtime: i64,
    pub size: i64,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexReport {
    pub indexed: usize,
    pub skipped: usize,
    pub removed: usize,
    /// One entry per page that could not be read. Such a page keeps whatever it
    /// already had in the index, so one broken file never blanks the search.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageHit {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub project: Option<String>,
    pub status: String,
    pub updated: String,
    /// Matched text from the body. Empty when the hit did not come from a query.
    pub snippet: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokenLink {
    pub src: String,
    pub dst: String,
}

fn stat(path: &Path) -> std::io::Result<FileStat> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(FileStat {
        mtime,
        size: meta.len() as i64,
    })
}

/// Bring the index in line with the files.
///
/// Takes `Db` rather than a `Connection` because the point of this routine is
/// when the lock is held. The walk, the `stat` calls and the parse all run with
/// it released, and the write takes it once. A sweep is disk work, and holding
/// the connection mutex across it would stall every other command for as long
/// as the disk takes.
pub fn sync(db: &Db, root: &Path) -> Result<IndexReport> {
    let current_version = INDEXER_VERSION.to_string();

    let (known, stored_version) = {
        let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
        (page_stats(&conn)?, meta_get(&conn, INDEXER_VERSION_KEY)?)
    };
    let reparse_everything = stored_version.as_deref() != Some(current_version.as_str());

    let mut report = IndexReport::default();
    let mut parsed: Vec<(Page, FileStat)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for path in wiki::walk(root)? {
        let id = wiki::page_id(root, &path)?;
        // Recorded before the stat, so a file that vanishes mid-sweep keeps its
        // rows rather than being treated as deleted.
        seen.insert(id.clone());

        let file = match stat(&path) {
            Ok(file) => file,
            Err(e) => {
                report.errors.push(format!("cannot read {id}: {e}"));
                continue;
            }
        };

        if !reparse_everything && known.get(&id) == Some(&file) {
            report.skipped += 1;
            continue;
        }

        match wiki::read(root, &path) {
            Ok(page) => parsed.push((page, file)),
            Err(e) => report.errors.push(e.to_string()),
        }
    }

    let mut conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
    let tx = conn.transaction()?;
    for (page, file) in &parsed {
        put_page(&tx, page, *file)?;
    }
    report.indexed = parsed.len();
    report.removed = remove_missing(&tx, &seen)?;
    meta_set(&tx, INDEXER_VERSION_KEY, &current_version)?;
    tx.commit()?;

    Ok(report)
}

/// Replace everything the index holds for one page.
fn put_page(tx: &Transaction, page: &Page, file: FileStat) -> Result<()> {
    // Sections and links cascade off this. `pages_fts` is a virtual table and
    // carries no foreign key, so it needs its own delete.
    tx.execute("DELETE FROM pages WHERE id = ?1", params![page.id])?;
    tx.execute("DELETE FROM pages_fts WHERE id = ?1", params![page.id])?;

    let fm = &page.frontmatter;
    tx.execute(
        "INSERT INTO pages
           (id, path, type, project, status, updated, title, body, mtime, size, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            page.id,
            page.path.to_string_lossy(),
            fm.page_type.as_str(),
            fm.project,
            fm.status.as_str(),
            fm.updated.to_string(),
            page.title,
            page.body,
            file.mtime,
            file.size,
            now(),
        ],
    )?;

    for (ord, section) in page.sections.iter().enumerate() {
        tx.execute(
            "INSERT INTO sections (page_id, ord, heading, body) VALUES (?1, ?2, ?3, ?4)",
            params![page.id, ord as i64, section.heading, section.body],
        )?;
    }

    for link in &page.links {
        // Prose repeats a link, the graph needs one edge.
        tx.execute(
            "INSERT OR IGNORE INTO links (src, dst) VALUES (?1, ?2)",
            params![page.id, link.target],
        )?;
    }

    tx.execute(
        "INSERT INTO pages_fts (id, title, body) VALUES (?1, ?2, ?3)",
        params![page.id, page.title, page.body],
    )?;

    Ok(())
}

/// Drop pages whose file is gone. This is what the delete trigger used to do
/// for `contexts`, moved into the sweep because the filesystem is the authority.
fn remove_missing(tx: &Transaction, seen: &HashSet<String>) -> Result<usize> {
    let indexed: Vec<String> = {
        let mut stmt = tx.prepare("SELECT id FROM pages")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()?
    };

    let mut removed = 0;
    for id in indexed.iter().filter(|id| !seen.contains(*id)) {
        tx.execute("DELETE FROM pages WHERE id = ?1", params![id])?;
        tx.execute("DELETE FROM pages_fts WHERE id = ?1", params![id])?;
        removed += 1;
    }
    Ok(removed)
}

fn page_stats(conn: &Connection) -> Result<HashMap<String, FileStat>> {
    let mut stmt = conn.prepare("SELECT id, mtime, size FROM pages")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            FileStat {
                mtime: row.get(1)?,
                size: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<HashMap<_, _>>>()?)
}

fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
            r.get(0)
        })
        .optional()?)
}

fn meta_set(tx: &Transaction, key: &str, value: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn row_to_hit(row: &rusqlite::Row) -> rusqlite::Result<PageHit> {
    Ok(PageHit {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        page_type: row.get(3)?,
        project: row.get(4)?,
        status: row.get(5)?,
        updated: row.get(6)?,
        snippet: row.get(7)?,
    })
}

const HIT_COLUMNS: &str = "p.id, p.path, p.title, p.type, p.project, p.status, p.updated";

pub fn list_pages(conn: &Connection) -> Result<Vec<PageHit>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {HIT_COLUMNS}, '' FROM pages p ORDER BY p.updated DESC, p.id"
    ))?;
    let rows = stmt.query_map([], row_to_hit)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Full-text search over the wiki.
///
/// The wiki keeps superseded and abandoned pages on purpose, so the first sort
/// term pushes them below live ones. Without it a reversed decision can outrank
/// the decision that replaced it.
pub fn search_pages(conn: &Connection, raw: &str) -> Result<Vec<PageHit>> {
    let Some(query) = to_fts_query(raw) else {
        return list_pages(conn);
    };

    let mut stmt = conn.prepare(&format!(
        "SELECT {HIT_COLUMNS}, snippet(pages_fts, 2, '', '', '...', 12)
         FROM pages_fts f
         JOIN pages p ON p.id = f.id
         WHERE pages_fts MATCH ?1
         ORDER BY (p.status = 'current') DESC,
                  bm25(pages_fts, 0.0, 10.0, 1.0),
                  p.updated DESC"
    ))?;
    let rows = stmt.query_map(params![query], row_to_hit)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Pages that link to `id`. The wiki's own answer to "what depends on this".
pub fn backlinks(conn: &Connection, id: &str) -> Result<Vec<PageHit>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {HIT_COLUMNS}, ''
         FROM links l
         JOIN pages p ON p.id = l.src
         WHERE l.dst = ?1
         ORDER BY p.id"
    ))?;
    let rows = stmt.query_map(params![id], row_to_hit)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Links that name a page which does not exist.
pub fn broken_links(conn: &Connection) -> Result<Vec<BrokenLink>> {
    let mut stmt = conn.prepare(
        "SELECT l.src, l.dst
         FROM links l
         LEFT JOIN pages p ON p.id = l.dst
         WHERE p.id IS NULL
         ORDER BY l.src, l.dst",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BrokenLink {
            src: row.get(0)?,
            dst: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn ctx(id: &str, name: &str, goal: &str) -> Context {
        Context {
            id: id.into(),
            name: name.into(),
            body: ContextBody {
                goal: Some(goal.into()),
                ..Default::default()
            },
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn save_then_get_roundtrips_the_body() {
        let c = mem();
        save(&c, &ctx("a", "Auth migration", "Replace auth with OAuth")).unwrap();
        let got = get(&c, "a").unwrap();
        assert_eq!(got.name, "Auth migration");
        assert_eq!(got.body.goal.as_deref(), Some("Replace auth with OAuth"));
        assert!(!got.created_at.is_empty());
    }

    #[test]
    fn save_is_upsert_and_preserves_created_at() {
        let c = mem();
        let first = save(&c, &ctx("a", "One", "g")).unwrap();
        let second = save(&c, &ctx("a", "Two", "g2")).unwrap();
        assert_eq!(second.name, "Two");
        assert_eq!(first.created_at, second.created_at, "created_at must survive");
        assert_eq!(list(&c).unwrap().len(), 1, "upsert, not insert");
    }

    #[test]
    fn search_matches_name_and_body_by_prefix() {
        let c = mem();
        save(&c, &ctx("a", "Auth migration", "OAuth work")).unwrap();
        save(&c, &ctx("b", "Stripe integration", "billing")).unwrap();

        // prefix of the name
        let hits = search(&c, "aut").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");

        // a word only present in the JSON body
        let hits = search(&c, "billing").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "b");
    }

    #[test]
    fn search_survives_punctuation_and_empty_input() {
        let c = mem();
        save(&c, &ctx("a", "Auth", "g")).unwrap();
        // Bare FTS syntax characters must not error or match wrongly.
        for q in ["", "   ", "\"", "*", "^-:", "AND"] {
            let _ = search(&c, q).expect("query must not error");
        }
        assert_eq!(search(&c, "").unwrap().len(), 1, "empty falls back to list");
    }

    #[test]
    fn fts_index_stays_in_sync_on_update_and_delete() {
        let c = mem();
        save(&c, &ctx("a", "Auth migration", "g")).unwrap();
        assert_eq!(search(&c, "auth").unwrap().len(), 1);

        // Renaming must remove the old term from the index.
        save(&c, &ctx("a", "Stripe integration", "g")).unwrap();
        assert!(search(&c, "auth").unwrap().is_empty(), "stale index entry");
        assert_eq!(search(&c, "stripe").unwrap().len(), 1);

        delete(&c, "a").unwrap();
        assert!(search(&c, "stripe").unwrap().is_empty());
    }

    #[test]
    fn delete_all_wipes_rows_and_index() {
        let c = mem();
        save(&c, &ctx("a", "Auth", "g")).unwrap();
        save(&c, &ctx("b", "Stripe", "g")).unwrap();
        delete_all(&c).unwrap();
        assert!(list(&c).unwrap().is_empty());
        assert!(search(&c, "auth").unwrap().is_empty());
    }

    #[test]
    fn sources_cascade_when_context_is_deleted() {
        let c = mem();
        save(&c, &ctx("a", "Auth", "g")).unwrap();
        c.execute(
            "INSERT INTO sources (id, context_id, type, content, created_at)
             VALUES ('s1', 'a', 'conversation', 'raw', '2026-01-01')",
            [],
        )
        .unwrap();
        delete(&c, "a").unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "orphaned source rows");
    }

    #[test]
    fn add_source_records_raw_text_against_the_context() {
        let c = mem();
        save(&c, &ctx("a", "Auth", "g")).unwrap();
        add_source(&c, "a", "conversation", "raw chat text").unwrap();
        let (kind, body): (String, String) = c
            .query_row(
                "SELECT type, content FROM sources WHERE context_id = 'a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(kind, "conversation");
        assert_eq!(body, "raw chat text");
    }

    #[test]
    fn missing_context_is_not_found_not_a_panic() {
        let c = mem();
        assert!(matches!(get(&c, "nope"), Err(DbError::NotFound(_))));
        assert!(matches!(delete(&c, "nope"), Err(DbError::NotFound(_))));
    }

    #[test]
    fn migrate_is_idempotent() {
        let c = mem();
        migrate(&c).unwrap();
        migrate(&c).unwrap();
        let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 2);
    }

    // ------------------------------------------------------------ wiki index

    use std::path::PathBuf;

    /// A throwaway wiki on disk plus an in-memory index over it.
    struct Fixture {
        root: PathBuf,
        db: Db,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("baton-index-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            Fixture {
                root,
                db: Db(Mutex::new(mem())),
            }
        }

        fn write(&self, id: &str, status: &str, body: &str) {
            let path = self.root.join(format!("{id}.md"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                format!(
                    "---\ntype: decision\nproject: baton\nstatus: {status}\n\
                     updated: 2026-08-22\nsources: [7123f71b]\n---\n\n{body}"
                ),
            )
            .unwrap();
        }

        fn sweep(&self) -> IndexReport {
            super::sync(&self.db, &self.root).unwrap()
        }

        fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> T {
            let conn = self.db.0.lock().unwrap_or_else(|e| e.into_inner());
            f(&conn).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn a_sweep_indexes_pages_sections_and_links() {
        let w = Fixture::new();
        w.write(
            "projects/baton/decisions/files-are-truth",
            "current",
            "# Markdown files are the source of truth\n\n\
             ## Decision\n\nPlain markdown in one folder.\n\n\
             ## Why\n\nGit gives history. See [[concepts/mutex-across-await]].\n",
        );
        w.write(
            "concepts/mutex-across-await",
            "current",
            "# Never hold a std Mutex across an await\n\n## Decision\n\nIt stalls every task.\n",
        );

        let report = w.sweep();
        assert_eq!((report.indexed, report.skipped, report.removed), (2, 0, 0));
        assert!(report.errors.is_empty());
        assert_eq!(w.read(|c| list_pages(c)).len(), 2);

        let hits = w.read(|c| search_pages(c, "git"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "projects/baton/decisions/files-are-truth");
        assert_eq!(hits[0].title, "Markdown files are the source of truth");
        assert_eq!(hits[0].project.as_deref(), Some("baton"));
        assert!(hits[0].snippet.to_lowercase().contains("git"));

        let back = w.read(|c| backlinks(c, "concepts/mutex-across-await"));
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, "projects/baton/decisions/files-are-truth");
    }

    #[test]
    fn a_second_sweep_skips_files_that_have_not_changed() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nOne.\n");
        assert_eq!(w.sweep().indexed, 1);

        let again = w.sweep();
        assert_eq!((again.indexed, again.skipped), (0, 1));
        assert_eq!(w.read(|c| list_pages(c)).len(), 1, "a skip must not drop the row");
    }

    #[test]
    fn an_edited_page_leaves_no_stale_section_and_no_stale_search_term() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nUse Postgres for everything.\n");
        w.sweep();
        assert_eq!(w.read(|c| search_pages(c, "postgres")).len(), 1);

        w.write("a", "current", "# A\n\n## Rejected\n\nUse SQLite, it is smaller and local.\n");
        assert_eq!(w.sweep().indexed, 1);

        assert!(
            w.read(|c| search_pages(c, "postgres")).is_empty(),
            "the FTS row for the old body is still there"
        );
        assert_eq!(w.read(|c| search_pages(c, "sqlite")).len(), 1);

        let headings: Vec<String> = w.read(|c| {
            let mut stmt =
                c.prepare("SELECT heading FROM sections WHERE page_id = 'a' ORDER BY ord")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
        });
        assert_eq!(headings, ["Rejected"], "the old section row survived");
    }

    #[test]
    fn a_deleted_file_is_removed_from_the_index_and_the_search() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nStripe billing.\n");
        w.write("b", "current", "# B\n\n## Decision\n\nAuth migration.\n");
        w.sweep();
        assert_eq!(w.read(|c| search_pages(c, "stripe")).len(), 1);

        std::fs::remove_file(w.root.join("a.md")).unwrap();
        let report = w.sweep();

        assert_eq!((report.indexed, report.skipped, report.removed), (0, 1, 1));
        assert!(w.read(|c| search_pages(c, "stripe")).is_empty());
        assert_eq!(w.read(|c| list_pages(c)).len(), 1);

        let sections: i64 =
            w.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM sections", [], |r| r.get(0))?));
        assert_eq!(sections, 1, "sections must cascade with the page");
    }

    #[test]
    fn a_malformed_page_is_reported_and_keeps_the_rows_it_had() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nKeep the launcher warm.\n");
        w.sweep();

        std::fs::write(w.root.join("a.md"), "someone deleted the frontmatter\n").unwrap();
        let report = w.sweep();

        assert_eq!(report.indexed, 0);
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("frontmatter"));
        assert_eq!(
            w.read(|c| search_pages(c, "launcher")).len(),
            1,
            "one unreadable file must not blank the index"
        );
    }

    #[test]
    fn a_parser_change_forces_a_rebuild_past_the_mtime_gate() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nOne.\n");
        w.sweep();
        assert_eq!(w.sweep().skipped, 1);

        // What bumping INDEXER_VERSION looks like from the index's side.
        {
            let mut conn = w.db.0.lock().unwrap_or_else(|e| e.into_inner());
            let tx = conn.transaction().unwrap();
            meta_set(&tx, INDEXER_VERSION_KEY, "0").unwrap();
            tx.commit().unwrap();
        }

        let report = w.sweep();
        assert_eq!(
            (report.indexed, report.skipped),
            (1, 0),
            "the mtime gate must yield to a parser change"
        );
    }

    #[test]
    fn search_ranks_a_live_page_above_the_one_it_replaced() {
        let w = Fixture::new();
        w.write(
            "old",
            "superseded",
            "# Old\n\n## Decision\n\nThe content column holds JSON, not markdown.\n",
        );
        w.write(
            "new",
            "current",
            "# New\n\n## Decision\n\nMarkdown files are the truth. JSON is gone.\n",
        );
        w.sweep();

        let hits = w.read(|c| search_pages(c, "json"));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "new", "a superseded page outranked the live one");
    }

    #[test]
    fn a_link_to_a_page_that_does_not_exist_is_reported_as_broken() {
        let w = Fixture::new();
        w.write(
            "a",
            "current",
            "# A\n\n## Decision\n\nSee [[open/browser-conversations]] and [[b]].\n",
        );
        w.write("b", "current", "# B\n\n## Decision\n\nHere.\n");
        w.sweep();

        let broken = w.read(|c| broken_links(c));
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].src, "a");
        assert_eq!(broken[0].dst, "open/browser-conversations");
    }

    #[test]
    fn delete_all_clears_the_index_and_leaves_the_wiki_files_alone() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nStripe billing.\n");
        w.sweep();

        w.read(|c| delete_all(c));
        assert!(w.read(|c| list_pages(c)).is_empty());
        assert!(w.read(|c| search_pages(c, "stripe")).is_empty());
        assert!(
            w.root.join("a.md").exists(),
            "the wiki files are not the index's to delete"
        );

        // And the index comes back from the files with no extra step.
        assert_eq!(w.sweep().indexed, 1);
    }
}
