//! Schema migrations.
//!
//! Each migration is applied exactly once. The version is tracked in a
//! `schema_version` table.

use rusqlite::{Connection, Result};

const CURRENT_VERSION: i32 = 1;

pub fn migrate(conn: &Connection) -> Result<()> {
    // Create the version tracking table if it doesn't exist.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
        [],
    )?;

    let version: i32 = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if version >= CURRENT_VERSION {
        return Ok(());
    }

    tracing::info!(from = version, to = CURRENT_VERSION, "migrating database");

    if version < 1 {
        apply_v1(conn)?;
    }

    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
        [CURRENT_VERSION],
    )?;

    Ok(())
}

fn apply_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE projects (
            id BLOB PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            is_git_repo INTEGER NOT NULL,
            current_branch TEXT,
            created_at TEXT NOT NULL,
            last_opened_at TEXT NOT NULL
        );

        CREATE TABLE threads (
            id BLOB PRIMARY KEY NOT NULL,
            project_id BLOB NOT NULL,
            title TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE runs (
            id BLOB PRIMARY KEY NOT NULL,
            thread_id BLOB NOT NULL,
            prompt TEXT NOT NULL,
            state TEXT NOT NULL,
            worktree_id BLOB,
            created_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
        );

        CREATE TABLE run_events (
            id BLOB PRIMARY KEY NOT NULL,
            run_id BLOB NOT NULL,
            sequence INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
        );

        CREATE TABLE worktrees (
            id BLOB PRIMARY KEY NOT NULL,
            project_id BLOB NOT NULL,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            branch TEXT NOT NULL,
            head_sha TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE terminal_sessions (
            id BLOB PRIMARY KEY NOT NULL,
            project_id BLOB NOT NULL,
            worktree_id BLOB,
            cwd TEXT NOT NULL,
            exit_code INTEGER,
            created_at TEXT NOT NULL,
            closed_at TEXT,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE artifacts (
            id BLOB PRIMARY KEY NOT NULL,
            run_id BLOB NOT NULL,
            artifact_type TEXT NOT NULL,
            file_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_threads_project ON threads(project_id);
        CREATE INDEX idx_runs_thread ON runs(thread_id);
        CREATE INDEX idx_run_events_run ON run_events(run_id, sequence);
        CREATE INDEX idx_worktrees_project ON worktrees(project_id);
        CREATE INDEX idx_terminal_sessions_project ON terminal_sessions(project_id);
        CREATE INDEX idx_artifacts_run ON artifacts(run_id);
        "#,
    )
}
