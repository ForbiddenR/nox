//! Thread persistence.

use models::{Thread, Uuid};
use rusqlite::{params, Connection, Result};

pub struct ThreadStore<'a> {
    conn: &'a Connection,
}

impl<'a> ThreadStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert a new thread.
    pub fn insert(&self, thread: &Thread) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO threads (id, project_id, title, archived, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                thread.id.as_bytes().as_slice(),
                thread.project_id.as_bytes().as_slice(),
                thread.title,
                thread.archived,
                thread.created_at.to_rfc3339(),
                thread.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Update an existing thread.
    pub fn update(&self, thread: &Thread) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE threads
            SET title = ?2, archived = ?3, updated_at = ?4
            WHERE id = ?1
            "#,
            params![
                thread.id.as_bytes().as_slice(),
                thread.title,
                thread.archived,
                thread.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get a thread by ID.
    pub fn get(&self, id: Uuid) -> Result<Option<Thread>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, archived, created_at, updated_at
             FROM threads WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.as_bytes().as_slice()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_thread(row)?))
        } else {
            Ok(None)
        }
    }

    /// List threads for a project, optionally excluding archived ones.
    pub fn list(&self, project_id: Uuid, include_archived: bool) -> Result<Vec<Thread>> {
        let sql = if include_archived {
            "SELECT id, project_id, title, archived, created_at, updated_at
             FROM threads WHERE project_id = ?1 ORDER BY updated_at DESC"
        } else {
            "SELECT id, project_id, title, archived, created_at, updated_at
             FROM threads WHERE project_id = ?1 AND archived = 0 ORDER BY updated_at DESC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![project_id.as_bytes().as_slice()], row_to_thread)?;
        rows.collect()
    }
}

fn row_to_thread(row: &rusqlite::Row) -> Result<Thread> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let project_id_bytes: Vec<u8> = row.get(1)?;
    let id = Uuid::from_slice(&id_bytes)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e)))?;
    let project_id = Uuid::from_slice(&project_id_bytes)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Blob, Box::new(e)))?;
    let created_at: String = row.get(4)?;
    let updated_at: String = row.get(5)?;

    Ok(Thread {
        id,
        project_id,
        title: row.get(2)?,
        archived: row.get(3)?,
        created_at: created_at.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?,
        updated_at: updated_at.parse().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chrono::Utc;
    use models::Project;

    #[test]
    fn inserts_and_retrieves_thread() {
        let db = Database::in_memory().unwrap();

        // Need a project first due to foreign key.
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

        let thread = Thread {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: "Test thread".into(),
            archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let store = db.threads();
        store.insert(&thread).unwrap();
        let retrieved = store.get(thread.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Test thread");
        assert!(!retrieved.archived);
    }

    #[test]
    fn lists_threads_for_project() {
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

        let t1 = Thread {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: "Active".into(),
            archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let t2 = Thread {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: "Archived".into(),
            archived: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let store = db.threads();
        store.insert(&t1).unwrap();
        store.insert(&t2).unwrap();

        let active = store.list(project.id, false).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].title, "Active");

        let all = store.list(project.id, true).unwrap();
        assert_eq!(all.len(), 2);
    }
}
