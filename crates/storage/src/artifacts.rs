//! Artifact storage: CRUD operations for run artifacts (diffs, patches, logs).

use crate::Database;
use models::{Artifact, ArtifactType, Uuid};
use rusqlite::{OptionalExtension, Result};

impl Database {
    /// Insert a new artifact.
    pub fn insert_artifact(&self, artifact: &Artifact) -> Result<()> {
        self.conn.execute(
            "INSERT INTO artifacts (id, run_id, artifact_type, file_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                artifact.id.as_bytes().as_slice(),
                artifact.run_id.as_bytes().as_slice(),
                serde_json::to_string(&artifact.artifact_type).unwrap(),
                &artifact.file_path,
                artifact.created_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    /// Get an artifact by ID.
    pub fn get_artifact(&self, artifact_id: Uuid) -> Result<Option<Artifact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, artifact_type, file_path, created_at
             FROM artifacts WHERE id = ?1",
        )?;

        let artifact = stmt
            .query_row([artifact_id.as_bytes().as_slice()], |row| {
                Ok(Artifact {
                    id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                    run_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                    artifact_type: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    file_path: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap()
                        .into(),
                })
            })
            .optional()?;

        Ok(artifact)
    }

    /// List artifacts for a run, optionally filtered by type.
    pub fn list_artifacts(
        &self,
        run_id: Uuid,
        artifact_type: Option<ArtifactType>,
    ) -> Result<Vec<Artifact>> {
        let query = if artifact_type.is_some() {
            "SELECT id, run_id, artifact_type, file_path, created_at
             FROM artifacts WHERE run_id = ?1 AND artifact_type = ?2 ORDER BY created_at DESC"
        } else {
            "SELECT id, run_id, artifact_type, file_path, created_at
             FROM artifacts WHERE run_id = ?1 ORDER BY created_at DESC"
        };

        let mut stmt = self.conn.prepare(query)?;

        let artifacts = if let Some(atype) = artifact_type {
            stmt.query_map(
                (
                    run_id.as_bytes().as_slice(),
                    serde_json::to_string(&atype).unwrap(),
                ),
                |row| {
                    Ok(Artifact {
                        id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                        run_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                        artifact_type: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                        file_path: row.get(3)?,
                        created_at: chrono::DateTime::parse_from_rfc3339(
                            &row.get::<_, String>(4)?,
                        )
                        .unwrap()
                        .into(),
                    })
                },
            )?
            .collect::<Result<Vec<_>>>()?
        } else {
            stmt.query_map([run_id.as_bytes().as_slice()], |row| {
                Ok(Artifact {
                    id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                    run_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                    artifact_type: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    file_path: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .unwrap()
                        .into(),
                })
            })?
            .collect::<Result<Vec<_>>>()?
        };

        Ok(artifacts)
    }

    /// Delete an artifact record (does not delete the file).
    pub fn delete_artifact(&self, artifact_id: Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM artifacts WHERE id = ?1",
            [artifact_id.as_bytes().as_slice()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{Project, Run, RunState, Thread};

    fn setup() -> (Database, Uuid) {
        let db = Database::in_memory().unwrap();

        let project = Project {
            id: Uuid::new_v4(),
            name: "test".into(),
            path: "/tmp/test".into(),
            is_git_repo: false,
            current_branch: None,
            created_at: chrono::Utc::now(),
            last_opened_at: chrono::Utc::now(),
        };
        db.projects().upsert(&project).unwrap();

        let thread = Thread {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: "Test thread".into(),
            archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        db.threads().insert(&thread).unwrap();

        let run = Run {
            id: Uuid::new_v4(),
            thread_id: thread.id,
            prompt: "test".into(),
            state: RunState::Completed,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };
        db.insert_run(&run).unwrap();

        (db, run.id)
    }

    #[test]
    fn inserts_and_retrieves_artifact() {
        let (db, run_id) = setup();

        let artifact = Artifact {
            id: Uuid::new_v4(),
            run_id,
            artifact_type: ArtifactType::Diff,
            file_path: "/tmp/test.diff".into(),
            created_at: chrono::Utc::now(),
        };

        db.insert_artifact(&artifact).unwrap();
        let retrieved = db.get_artifact(artifact.id).unwrap().unwrap();

        assert_eq!(retrieved.id, artifact.id);
        assert_eq!(retrieved.file_path, "/tmp/test.diff");
    }

    #[test]
    fn lists_artifacts_for_run() {
        let (db, run_id) = setup();

        let diff = Artifact {
            id: Uuid::new_v4(),
            run_id,
            artifact_type: ArtifactType::Diff,
            file_path: "/tmp/test.diff".into(),
            created_at: chrono::Utc::now(),
        };
        db.insert_artifact(&diff).unwrap();

        let patch = Artifact {
            id: Uuid::new_v4(),
            run_id,
            artifact_type: ArtifactType::Patch,
            file_path: "/tmp/test.patch".into(),
            created_at: chrono::Utc::now(),
        };
        db.insert_artifact(&patch).unwrap();

        let artifacts = db.list_artifacts(run_id, None).unwrap();
        assert_eq!(artifacts.len(), 2);

        let diffs = db.list_artifacts(run_id, Some(ArtifactType::Diff)).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].artifact_type, ArtifactType::Diff);
    }
}
