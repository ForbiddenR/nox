//! Worktree persistence operations.

use anyhow::Result;
use models::Worktree;
use rusqlite::{params, Connection};
use uuid::Uuid;

/// Insert a new worktree.
pub fn insert_worktree(conn: &Connection, worktree: &Worktree) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO worktrees (id, project_id, name, path, branch, head_sha, is_active, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            worktree.id.as_bytes().as_slice(),
            worktree.project_id.as_bytes().as_slice(),
            worktree.name,
            worktree.path,
            worktree.branch,
            worktree.head_sha,
            worktree.is_active,
            worktree.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// List all worktrees for a project.
pub fn list_worktrees(conn: &Connection, project_id: Uuid) -> Result<Vec<Worktree>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, project_id, name, path, branch, head_sha, is_active, created_at
        FROM worktrees
        WHERE project_id = ?1
        ORDER BY created_at DESC
        "#,
    )?;

    let rows = stmt.query_map([project_id.as_bytes().as_slice()], |row| {
        let id_bytes: Vec<u8> = row.get(0)?;
        let project_id_bytes: Vec<u8> = row.get(1)?;
        let created_at_str: String = row.get(7)?;

        Ok(Worktree {
            id: Uuid::from_slice(&id_bytes).unwrap(),
            project_id: Uuid::from_slice(&project_id_bytes).unwrap(),
            name: row.get(2)?,
            path: row.get(3)?,
            branch: row.get(4)?,
            head_sha: row.get(5)?,
            is_active: row.get(6)?,
            created_at: created_at_str.parse().unwrap(),
        })
    })?;

    let mut worktrees = Vec::new();
    for row_result in rows {
        worktrees.push(row_result?);
    }

    Ok(worktrees)
}

/// Get a single worktree by ID.
pub fn get_worktree(conn: &Connection, worktree_id: Uuid) -> Result<Option<Worktree>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, project_id, name, path, branch, head_sha, is_active, created_at
        FROM worktrees
        WHERE id = ?1
        "#,
    )?;

    let result = stmt.query_row([worktree_id.as_bytes().as_slice()], |row| {
        let id_bytes: Vec<u8> = row.get(0)?;
        let project_id_bytes: Vec<u8> = row.get(1)?;
        let created_at_str: String = row.get(7)?;

        Ok(Worktree {
            id: Uuid::from_slice(&id_bytes).unwrap(),
            project_id: Uuid::from_slice(&project_id_bytes).unwrap(),
            name: row.get(2)?,
            path: row.get(3)?,
            branch: row.get(4)?,
            head_sha: row.get(5)?,
            is_active: row.get(6)?,
            created_at: created_at_str.parse().unwrap(),
        })
    });

    match result {
        Ok(worktree) => Ok(Some(worktree)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Set a worktree as active (and deactivate all others for the same project).
pub fn set_active_worktree(conn: &Connection, worktree_id: Uuid) -> Result<()> {
    // Get the project_id first
    let worktree = get_worktree(conn, worktree_id)?
        .ok_or_else(|| anyhow::anyhow!("Worktree not found"))?;

    // Deactivate all worktrees for this project
    conn.execute(
        "UPDATE worktrees SET is_active = 0 WHERE project_id = ?1",
        [worktree.project_id.as_bytes().as_slice()],
    )?;

    // Activate the selected worktree
    conn.execute(
        "UPDATE worktrees SET is_active = 1 WHERE id = ?1",
        [worktree_id.as_bytes().as_slice()],
    )?;

    Ok(())
}

/// Delete a worktree record.
pub fn delete_worktree(conn: &Connection, worktree_id: Uuid) -> Result<()> {
    conn.execute(
        "DELETE FROM worktrees WHERE id = ?1",
        [worktree_id.as_bytes().as_slice()],
    )?;
    Ok(())
}

/// Update worktree HEAD SHA.
pub fn update_worktree_head(conn: &Connection, worktree_id: Uuid, head_sha: &str) -> Result<()> {
    conn.execute(
        "UPDATE worktrees SET head_sha = ?1 WHERE id = ?2",
        params![head_sha, worktree_id.as_bytes().as_slice()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrations::migrate(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection) -> Uuid {
        let project_id = Uuid::new_v4();
        conn.execute(
            r#"
            INSERT INTO projects (id, name, path, is_git_repo, current_branch, created_at, last_opened_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                project_id.as_bytes().as_slice(),
                "Test Project",
                "/tmp/test",
                1,
                "main",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();
        project_id
    }

    #[test]
    fn test_insert_and_list_worktrees() {
        let conn = setup_db();
        let project_id = create_test_project(&conn);

        let worktree = Worktree {
            id: Uuid::new_v4(),
            project_id,
            name: "feature-1".to_string(),
            path: "/tmp/worktrees/feature-1".to_string(),
            branch: "cox/feature-1".to_string(),
            head_sha: "abc123".to_string(),
            is_active: false,
            created_at: Utc::now(),
        };

        insert_worktree(&conn, &worktree).unwrap();

        let worktrees = list_worktrees(&conn, project_id).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].name, "feature-1");
    }

    #[test]
    fn test_get_worktree() {
        let conn = setup_db();
        let project_id = create_test_project(&conn);

        let worktree_id = Uuid::new_v4();
        let worktree = Worktree {
            id: worktree_id,
            project_id,
            name: "feature-2".to_string(),
            path: "/tmp/worktrees/feature-2".to_string(),
            branch: "cox/feature-2".to_string(),
            head_sha: "def456".to_string(),
            is_active: false,
            created_at: Utc::now(),
        };

        insert_worktree(&conn, &worktree).unwrap();

        let retrieved = get_worktree(&conn, worktree_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "feature-2");
    }

    #[test]
    fn test_set_active_worktree() {
        let conn = setup_db();
        let project_id = create_test_project(&conn);

        let wt1 = Worktree {
            id: Uuid::new_v4(),
            project_id,
            name: "feature-1".to_string(),
            path: "/tmp/worktrees/feature-1".to_string(),
            branch: "cox/feature-1".to_string(),
            head_sha: "abc123".to_string(),
            is_active: true,
            created_at: Utc::now(),
        };

        let wt2 = Worktree {
            id: Uuid::new_v4(),
            project_id,
            name: "feature-2".to_string(),
            path: "/tmp/worktrees/feature-2".to_string(),
            branch: "cox/feature-2".to_string(),
            head_sha: "def456".to_string(),
            is_active: false,
            created_at: Utc::now(),
        };

        insert_worktree(&conn, &wt1).unwrap();
        insert_worktree(&conn, &wt2).unwrap();

        // Set wt2 as active
        set_active_worktree(&conn, wt2.id).unwrap();

        let updated_wt1 = get_worktree(&conn, wt1.id).unwrap().unwrap();
        let updated_wt2 = get_worktree(&conn, wt2.id).unwrap().unwrap();

        assert!(!updated_wt1.is_active);
        assert!(updated_wt2.is_active);
    }

    #[test]
    fn test_delete_worktree() {
        let conn = setup_db();
        let project_id = create_test_project(&conn);

        let worktree_id = Uuid::new_v4();
        let worktree = Worktree {
            id: worktree_id,
            project_id,
            name: "feature-3".to_string(),
            path: "/tmp/worktrees/feature-3".to_string(),
            branch: "cox/feature-3".to_string(),
            head_sha: "ghi789".to_string(),
            is_active: false,
            created_at: Utc::now(),
        };

        insert_worktree(&conn, &worktree).unwrap();
        delete_worktree(&conn, worktree_id).unwrap();

        let retrieved = get_worktree(&conn, worktree_id).unwrap();
        assert!(retrieved.is_none());
    }
}
