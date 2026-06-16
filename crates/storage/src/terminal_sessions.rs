//! Terminal session persistence.

use crate::Database;
use chrono::Utc;
use models::{TerminalSession, Uuid};
use rusqlite::{params, Result};

impl Database {
    /// Get the terminal sessions store.
    pub fn terminal_sessions(&self) -> TerminalSessionStore<'_> {
        TerminalSessionStore { db: self }
    }
}

pub struct TerminalSessionStore<'a> {
    db: &'a Database,
}

impl TerminalSessionStore<'_> {
    /// Insert a new terminal session.
    pub fn insert(&self, session: &TerminalSession) -> Result<()> {
        self.db.conn.execute(
            "INSERT INTO terminal_sessions (id, project_id, worktree_id, cwd, exit_code, created_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session.id.as_bytes().as_slice(),
                session.project_id.as_bytes().as_slice(),
                session.worktree_id.map(|id| id.as_bytes().to_vec()),
                session.cwd,
                session.exit_code,
                session.created_at.to_rfc3339(),
                session.closed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Get a terminal session by ID.
    pub fn get(&self, id: Uuid) -> Result<Option<TerminalSession>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, project_id, worktree_id, cwd, exit_code, created_at, closed_at
             FROM terminal_sessions WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id.as_bytes().as_slice()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TerminalSession {
                id: Uuid::from_slice(&row.get::<_, Vec<u8>>(0)?).unwrap(),
                project_id: Uuid::from_slice(&row.get::<_, Vec<u8>>(1)?).unwrap(),
                worktree_id: row.get::<_, Option<Vec<u8>>>(2)?.map(|b| Uuid::from_slice(&b).unwrap()),
                cwd: row.get(3)?,
                exit_code: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                closed_at: row
                    .get::<_, Option<String>>(6)?
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update a terminal session (typically to set exit_code and closed_at).
    pub fn update(&self, session: &TerminalSession) -> Result<()> {
        self.db.conn.execute(
            "UPDATE terminal_sessions
             SET exit_code = ?1, closed_at = ?2
             WHERE id = ?3",
            params![
                session.exit_code,
                session.closed_at.map(|dt| dt.to_rfc3339()),
                session.id.as_bytes().as_slice(),
            ],
        )?;
        Ok(())
    }

    /// List active terminal sessions for a project.
    pub fn list_active(&self, project_id: Uuid) -> Result<Vec<TerminalSession>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, project_id, worktree_id, cwd, exit_code, created_at, closed_at
             FROM terminal_sessions
             WHERE project_id = ?1 AND closed_at IS NULL
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(params![project_id.as_bytes().as_slice()], |row| {
            Ok(TerminalSession {
                id: Uuid::from_slice(&row.get::<_, Vec<u8>>(0)?).unwrap(),
                project_id: Uuid::from_slice(&row.get::<_, Vec<u8>>(1)?).unwrap(),
                worktree_id: row.get::<_, Option<Vec<u8>>>(2)?.map(|b| Uuid::from_slice(&b).unwrap()),
                cwd: row.get(3)?,
                exit_code: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                closed_at: row
                    .get::<_, Option<String>>(6)?
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
            })
        })?;

        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::Project;

    fn setup() -> (Database, Uuid) {
        let db = Database::in_memory().unwrap();

        let project = Project {
            id: Uuid::new_v4(),
            name: "test".into(),
            path: "/tmp/test".into(),
            is_git_repo: false,
            current_branch: None,
            created_at: Utc::now(),
            last_opened_at: Utc::now(),
        };
        db.projects().upsert(&project).unwrap();

        (db, project.id)
    }

    #[test]
    fn inserts_and_retrieves_session() {
        let (db, project_id) = setup();
        let store = db.terminal_sessions();

        let session = TerminalSession {
            id: Uuid::new_v4(),
            project_id,
            worktree_id: None,
            cwd: "/tmp/test".into(),
            exit_code: None,
            created_at: Utc::now(),
            closed_at: None,
        };

        store.insert(&session).unwrap();
        let retrieved = store.get(session.id).unwrap().unwrap();

        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.cwd, session.cwd);
    }

    #[test]
    fn lists_active_sessions() {
        let (db, project_id) = setup();
        let store = db.terminal_sessions();

        let session1 = TerminalSession {
            id: Uuid::new_v4(),
            project_id,
            worktree_id: None,
            cwd: "/tmp/test".into(),
            exit_code: None,
            created_at: Utc::now(),
            closed_at: None,
        };

        let session2 = TerminalSession {
            id: Uuid::new_v4(),
            project_id,
            worktree_id: None,
            cwd: "/tmp/test".into(),
            exit_code: Some(0),
            created_at: Utc::now(),
            closed_at: Some(Utc::now()),
        };

        store.insert(&session1).unwrap();
        store.insert(&session2).unwrap();

        let active = store.list_active(project_id).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, session1.id);
    }

    #[test]
    fn updates_session() {
        let (db, project_id) = setup();
        let store = db.terminal_sessions();

        let mut session = TerminalSession {
            id: Uuid::new_v4(),
            project_id,
            worktree_id: None,
            cwd: "/tmp/test".into(),
            exit_code: None,
            created_at: Utc::now(),
            closed_at: None,
        };

        store.insert(&session).unwrap();

        session.exit_code = Some(0);
        session.closed_at = Some(Utc::now());
        store.update(&session).unwrap();

        let updated = store.get(session.id).unwrap().unwrap();
        assert_eq!(updated.exit_code, Some(0));
        assert!(updated.closed_at.is_some());
    }
}
