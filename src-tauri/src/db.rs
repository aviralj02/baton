//! SQLite storage. Owned entirely by Rust — the webview never touches it.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, Transaction};

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
    ensure_schema(&conn)?;
    Ok(conn)
}

const SCHEMA: &str = r#"
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

-- `dst` is deliberately not a foreign key: a link may name a page that does
-- not exist, and finding those is the point.
CREATE TABLE links (
  src TEXT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
  dst TEXT NOT NULL,
  PRIMARY KEY (src, dst)
);

CREATE INDEX idx_links_dst ON links(dst);

CREATE VIRTUAL TABLE pages_fts USING fts5(id UNINDEXED, title, body);
"#;

/// Bump when `wiki.rs` starts producing different output from the same file.
/// The schema itself needs no such constant — it is fingerprinted below — but a
/// parser change leaves the schema identical while making every stored row
/// stale, and nothing else would notice.
const PARSER_VERSION: u64 = 1;

/// A stable fingerprint of what the index *is*: its shape plus the parser that
/// filled it. FNV-1a rather than `DefaultHasher`, whose output is explicitly
/// not stable across Rust releases and would silently rebuild the index on a
/// toolchain upgrade.
fn fingerprint() -> i32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in SCHEMA.as_bytes().iter().chain(&PARSER_VERSION.to_le_bytes()) {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    // `user_version` is i32; the low bits are as good as any.
    h as i32
}

/// The tables `SCHEMA` creates. Anything else in the file is an orphan from an
/// older build and is dropped.
const EXPECTED_TABLES: [&str; 4] = ["pages", "sections", "links", "pages_fts"];

fn ensure_schema(conn: &Connection) -> Result<()> {
    let stored: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let found = user_tables(conn)?;

    let fresh = found.is_empty();
    let orphans: Vec<&String> = found
        .iter()
        .filter(|n| !EXPECTED_TABLES.contains(&n.as_str()))
        .collect();

    if !fresh && stored == fingerprint() && orphans.is_empty() {
        return Ok(());
    }

    // Drop what is actually there, not what we remember putting there.
    // `pages_fts` must go before `pages`, and children before parents.
    let mut order: Vec<&String> = found.iter().collect();
    order.sort_by_key(|n| match n.as_str() {
        "pages_fts" => 0,
        "links" | "sections" => 1,
        "pages" => 3,
        _ => 2,
    });
    for name in order {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{name}\";"))?;
    }

    conn.execute_batch(SCHEMA)?;
    // Cannot be bound: PRAGMA takes no parameters.
    conn.execute_batch(&format!("PRAGMA user_version = {};", fingerprint()))?;
    Ok(())
}

/// Real tables in the file, excluding SQLite's own bookkeeping and the shadow
/// tables an FTS5 virtual table owns — those go when their parent goes.
fn user_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master
         WHERE type = 'table'
           AND name NOT LIKE 'sqlite_%'
           AND name NOT LIKE '%\\_fts\\_%' ESCAPE '\\'",
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ----------------------------------------------------------- the wiki index
//
// Everything below is derived from the markdown files under ~/Baton. None of it
// is authoritative, and deleting the database costs nothing but a re-sweep.

/// Bump this when `wiki.rs` changes the shape of what a parse produces.
///
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
    // A parser or schema change drops the whole index (see `ensure_schema`), so
    // `known` is simply empty then and everything reparses. No separate
    // "reparse everything" flag is needed.
    let known = {
        let conn = db.0.lock().unwrap_or_else(|e| e.into_inner());
        page_stats(&conn)?
    };

    let mut report = IndexReport::default();
    let mut parsed: Vec<(Page, FileStat)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let paths = match wiki::walk(root) {
        Ok(paths) => paths,
        Err(_) if !root.is_dir() => Vec::new(),
        Err(e) => return Err(e.into()),
    };

    for path in paths {
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

        if known.get(&id) == Some(&file) {
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

/// One row per project, for the launcher.
///
/// The launcher deals in projects, not pages. A project's pages are an
/// organisational detail of the wiki folder; what a user summons Baton for is
/// "give me everything about X", and splitting that across eight rows makes the
/// one action they want compete with seven they do not.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHit {
    pub slug: String,
    /// Title of the project's overview page, falling back to the slug.
    pub title: String,
    pub page_count: usize,
    /// Newest `updated` across the project's pages.
    pub updated: String,
}

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<ProjectHit> {
    let slug: String = row.get(0)?;
    let title: Option<String> = row.get(1)?;
    Ok(ProjectHit {
        title: title.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| slug.clone()),
        slug,
        page_count: row.get::<_, i64>(2)? as usize,
        updated: row.get(3)?,
    })
}

/// Projects, most recently touched first.
pub fn list_projects(conn: &Connection) -> Result<Vec<ProjectHit>> {
    let mut stmt = conn.prepare(
        "SELECT p.project,
                MAX(CASE WHEN p.type = 'project' THEN p.title END),
                COUNT(*),
                MAX(p.updated)
         FROM pages p
         WHERE p.project IS NOT NULL
         GROUP BY p.project
         ORDER BY MAX(p.updated) DESC, p.project",
    )?;
    let rows = stmt.query_map([], row_to_project)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Projects matching a query on the project name or any page title.
///
/// Deliberately not a body search. Matching body text would surface a project
/// because one sentence buried in one page mentioned the word, which reads as a
/// false positive when the row shown is the whole project.
pub fn search_projects(conn: &Connection, raw: &str) -> Result<Vec<ProjectHit>> {
    let needle = raw.trim().to_lowercase();
    if needle.is_empty() {
        return list_projects(conn);
    }

    let like = format!("%{}%", needle.replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = conn.prepare(
        "SELECT p.project,
                MAX(CASE WHEN p.type = 'project' THEN p.title END),
                COUNT(*),
                MAX(p.updated)
         FROM pages p
         WHERE p.project IS NOT NULL
           AND p.project IN (
             SELECT project FROM pages
             WHERE project IS NOT NULL
               AND (LOWER(project) LIKE ?1 ESCAPE '\\'
                    OR LOWER(title) LIKE ?1 ESCAPE '\\')
           )
         GROUP BY p.project
         ORDER BY MAX(p.updated) DESC, p.project",
    )?;
    let rows = stmt.query_map(params![like], row_to_project)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_pages(conn: &Connection) -> Result<Vec<PageHit>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {HIT_COLUMNS}, '' FROM pages p ORDER BY p.updated DESC, p.id"
    ))?;
    let rows = stmt.query_map([], row_to_hit)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Empty the index. The wiki files are untouched — they are the source of
/// truth and this is only the derived copy, so the next sweep restores
/// everything.
pub fn delete_all(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM pages;
         DELETE FROM sections;
         DELETE FROM links;
         DELETE FROM pages_fts;",
    )?;
    Ok(())
}

/// Turn raw user typing into a safe FTS5 prefix query.
///
/// User input can never reach the FTS parser directly: characters like `"`,
/// `*`, `^`, `-` and `:` are query syntax and would either error or silently
/// mean something else. Each run of alphanumerics becomes a quoted prefix term,
/// and terms are ANDed (space is AND in FTS5).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        ensure_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn ensure_schema_is_idempotent() {
        let c = mem();
        ensure_schema(&c).unwrap();
        ensure_schema(&c).unwrap();
        let v: i32 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, fingerprint());
    }

    #[test]
    fn a_changed_fingerprint_rebuilds_from_empty() {
        // The index carries no fact of its own, so a schema or parser change
        // throws it away rather than migrating it. Anything still needed comes
        // back from the files on the next sweep.
        let c = mem();
        c.execute(
            "INSERT INTO pages (id, path, type, project, status, updated, title, body, mtime, size, indexed_at)
             VALUES ('a','a.md','decision','baton','current','2026-08-23','A','b',0,0,'2026-08-23')",
            [],
        )
        .unwrap();
        assert_eq!(list_pages(&c).unwrap().len(), 1);

        // Simulate the code moving on.
        c.execute_batch("PRAGMA user_version = 999999;").unwrap();
        ensure_schema(&c).unwrap();

        assert!(list_pages(&c).unwrap().is_empty(), "stale rows survived");
        let v: i32 = c.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, fingerprint());
    }

    #[test]
    fn the_fingerprint_is_stable_across_runs() {
        // It is persisted, so a hash that varies per process would rebuild the
        // index on every launch. `DefaultHasher` does exactly that, which is
        // why this is FNV-1a.
        assert_eq!(fingerprint(), fingerprint());
        assert_ne!(fingerprint(), 0);
    }

    #[test]
    fn a_rebuild_leaves_exactly_the_current_schema() {
        // A table dropped from SCHEMA must also be dropped from an existing
        // file, or old installs keep an orphan forever while fresh ones do not.
        let c = mem();
        c.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        c.execute_batch("PRAGMA user_version = 1;").unwrap();
        ensure_schema(&c).unwrap();

        let leftover: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'meta'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(leftover, 0, "a table SCHEMA no longer creates survived");
    }

    #[test]
    fn a_fresh_database_has_no_pre_wiki_tables() {
        let c = mem();
        let names: Vec<String> = {
            let mut stmt = c
                .prepare("SELECT name FROM sqlite_master WHERE name IN ('contexts','sources','contexts_fts')")
                .unwrap();
            let rows = stmt.query_map([], |r| r.get(0)).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        assert!(names.is_empty(), "legacy tables present: {names:?}");
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

        /// Write a page with an explicit type and project — the plain `write`
        /// hardcodes `decision`/`baton`, which cannot express a project row.
        fn write_as(&self, id: &str, page_type: &str, project: &str, body: &str) {
            let path = self.root.join(format!("{id}.md"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                format!(
                    "---\ntype: {page_type}\nproject: {project}\nstatus: current\n\
                     updated: 2026-08-22\nsources: [7123f71b]\n---\n\n{body}"
                ),
            )
            .unwrap();
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
        assert_eq!(w.read(list_pages).len(), 2);

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
        assert_eq!(w.read(list_pages).len(), 1, "a skip must not drop the row");
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
        assert_eq!(w.read(list_pages).len(), 1);

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
    fn deleting_the_wiki_folder_empties_the_index() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nOne.\n");
        w.sweep();
        assert_eq!(w.read(list_pages).len(), 1);

        std::fs::remove_dir_all(&w.root).unwrap();
        let report = w.sweep();

        assert_eq!(report.removed, 1, "the vanished page was not removed");
        assert!(w.read(list_pages).is_empty());
        assert!(w.read(list_projects).is_empty());
    }

    #[test]
    fn a_parser_change_forces_a_rebuild_past_the_mtime_gate() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nOne.\n");
        w.sweep();
        // Unchanged file, so the mtime gate skips it.
        assert_eq!(w.sweep().skipped, 1);

        // What bumping PARSER_VERSION looks like from the index's side: the
        // fingerprint no longer matches, so the schema and every row go.
        {
            let conn = w.db.0.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute_batch("PRAGMA user_version = 424242;").unwrap();
            ensure_schema(&conn).unwrap();
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
    fn projects_group_their_pages_into_one_row() {
        let w = Fixture::new();
        w.write_as("projects/baton/overview", "project", "baton",
            "# Baton\n\n## Goal\n\nA launcher.\n\n## Current state\n\nx\n\n## Next step\n\ny\n");
        w.write_as("projects/baton/decisions/d", "decision", "baton",
            "# A decision\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n");
        w.sweep();

        let rows = w.read(list_projects);
        assert_eq!(rows.len(), 1, "two pages of one project must be one row");
        assert_eq!(rows[0].slug, "baton");
        assert_eq!(rows[0].page_count, 2);
        // The row is titled by the overview, not by the slug, when one exists.
        assert_eq!(rows[0].title, "Baton");
    }

    #[test]
    fn a_project_with_no_overview_falls_back_to_its_slug() {
        let w = Fixture::new();
        w.write_as("projects/orphaned/decisions/d", "decision", "orphaned",
            "# A decision\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n");
        w.sweep();
        let rows = w.read(list_projects);
        assert_eq!(rows[0].title, "orphaned");
    }

    #[test]
    fn search_matches_the_project_name_or_a_page_title() {
        let w = Fixture::new();
        w.write_as("projects/baton/overview", "project", "baton",
            "# Baton\n\n## Goal\n\nx\n\n## Current state\n\ny\n\n## Next step\n\nz\n");
        w.write_as("projects/baton/decisions/nspanel", "decision", "baton",
            "# Use a non-activating NSPanel\n\n## Decision\n\nx\n\n## Why\n\ny\n\n## Rejected\n\nz\n");
        w.write_as("projects/other/overview", "project", "other",
            "# Other\n\n## Goal\n\nx\n\n## Current state\n\ny\n\n## Next step\n\nz\n");
        w.sweep();

        // By project name.
        let hits = w.read(|c| search_projects(c, "bat"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].slug, "baton");

        // By a page title inside it — the project is the row, not the page.
        let hits = w.read(|c| search_projects(c, "nspanel"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].slug, "baton");
        assert_eq!(hits[0].page_count, 2, "the whole project comes back");

        // Empty query lists everything.
        assert_eq!(w.read(|c| search_projects(c, "  ")).len(), 2);
    }

    #[test]
    fn search_does_not_match_on_body_text() {
        // Matching a word buried in one page would surface the whole project,
        // which reads as a false positive when the row shown is the project.
        let w = Fixture::new();
        w.write_as("projects/baton/overview", "project", "baton",
            "# Baton\n\n## Goal\n\nSomething about kubernetes.\n\n## Current state\n\ny\n\n## Next step\n\nz\n");
        w.sweep();
        assert!(w.read(|c| search_projects(c, "kubernetes")).is_empty());
    }

    #[test]
    fn concepts_pages_are_not_a_project_row() {
        // They belong to no project on purpose, and are folded into every
        // brief rather than standing alone in the list.
        let w = Fixture::new();
        w.write_as("concepts/g", "gotcha", "null",
            "# G\n\n## The constraint\n\na\n\n## The symptom\n\nb\n\n## The fix\n\nc\n");
        w.sweep();
        assert!(w.read(list_projects).is_empty());
    }

    #[test]
    fn delete_all_clears_the_index_and_leaves_the_wiki_files_alone() {
        let w = Fixture::new();
        w.write("a", "current", "# A\n\n## Decision\n\nStripe billing.\n");
        w.sweep();

        w.read(delete_all);
        assert!(w.read(list_pages).is_empty());
        assert!(w.read(|c| search_pages(c, "stripe")).is_empty());
        assert!(
            w.root.join("a.md").exists(),
            "the wiki files are not the index's to delete"
        );

        // And the index comes back from the files with no extra step.
        assert_eq!(w.sweep().indexed, 1);
    }
}
