//! Project persistence.

use models::{Project, Uuid};
use rusqlite::{params, Connection, Result};

pub struct ProjectStore<'a> {
    conn: &'a Connection,
}

impl<'a> ProjectStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert or update a project.
    pub fn upsert(&self, project: &Project) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO projects (id, name, path, is_git_repo, current_branch, created_at, last_opened_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                is_git_repo = excluded.is_git_repo,
                current_branch = excluded.current_branch,
                last_opened_at = excluded.last_opened_at
            "#,
            params![
                project.id.as_bytes().as_slice(),
                project.name,
                project.path,
                project.is_git_repo,
                project.current_branch,
                project.created_at.to_rfc3339(),
                project.last_opened_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get a project by ID.
    pub fn get(&self, id: Uuid) -> Result<Option<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, is_git_repo, current_branch, created_at, last_opened_at
             FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_bytes().as_slice()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_project(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get a project by path.
    pub fn get_by_path(&self, path: &str) -> Result<Option<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, is_git_repo, current_branch, created_at, last_opened_at
             FROM projects WHERE path = ?1",
        )?;
        let mut rows = stmt.query(params![path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_project(row)?))
        } else {
            Ok(None)
        }
    }

    /// List recent projects, ordered by last opened descending.
    pub fn list_recent(&self, limit: usize) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, is_git_repo, current_branch, created_at, last_opened_at
             FROM projects ORDER BY last_opened_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], row_to_project)?;
        rows.collect()
    }
}

fn row_to_project(row: &rusqlite::Row) -> Result<Project> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let id = Uuid::from_slice(&id_bytes)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e)))?;
    let created_at: String = row.get(5)?;
    let last_opened_at: String = row.get(6)?;

    Ok(Project {
        id,
        name: row.get(1)?,
        path: row.get(2)?,
        is_git_repo: row.get(3)?,
        current_branch: row.get(4)?,
        created_at: created_at.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?,
        last_opened_at: last_opened_at.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chrono::Utc;

    #[test]
    fn upserts_and_retrieves_project() {
        let db = Database::in_memory().unwrap();
        let store = db.projects();

        let project = Project {
            id: Uuid::new_v4(),
            name: "test-project".into(),
            path: "/tmp/test".into(),
            is_git_repo: true,
            current_branch: Some("main".into()),
            created_at: Utc::now(),
            last_opened_at: Utc::now(),
        };

        store.upsert(&project).unwrap();
        let retrieved = store.get(project.id).unwrap().unwrap();
        assert_eq!(retrieved.name, "test-project");
        assert_eq!(retrieved.path, "/tmp/test");
    }

    #[test]
    fn lists_recent_projects() {
        let db = Database::in_memory().unwrap();
        let store = db.projects();

        let p1 = Project {
            id: Uuid::new_v4(),
            name: "older".into(),
            path: "/tmp/older".into(),
            is_git_repo: false,
            current_branch: None,
            created_at: Utc::now(),
            last_opened_at: Utc::now(),
        };

        let p2 = Project {
            id: Uuid::new_v4(),
            name: "newer".into(),
            path: "/tmp/newer".into(),
            is_git_repo: true,
            current_branch: Some("main".into()),
            created_at: Utc::now(),
            last_opened_at: Utc::now(),
        };

        store.upsert(&p1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.upsert(&p2).unwrap();

        let recent = store.list_recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].name, "newer");
        assert_eq!(recent[1].name, "older");
    }
}
