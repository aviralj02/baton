//! SQLite storage. Owned entirely by Rust — the webview never touches it.

use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::context::{Context, ContextBody, ContextSummary};

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

pub fn delete_all(conn: &Connection) -> Result<()> {
    conn.execute_batch("DELETE FROM contexts; DELETE FROM sources;")?;
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
        assert_eq!(v, 1);
    }
}
