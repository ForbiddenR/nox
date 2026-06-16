//! Run storage: CRUD operations for runs and run events.

use crate::Database;
use models::{Run, RunEvent, Uuid};
use rusqlite::{OptionalExtension, Result};

impl Database {
    /// Insert a new run.
    pub fn insert_run(&self, run: &Run) -> Result<()> {
        self.conn.execute(
            "INSERT INTO runs (id, thread_id, prompt, state, worktree_id, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                run.id.as_bytes().as_slice(),
                run.thread_id.as_bytes().as_slice(),
                &run.prompt,
                serde_json::to_string(&run.state).unwrap(),
                run.worktree_id.map(|id| id.as_bytes().to_vec()),
                run.created_at.to_rfc3339(),
                run.completed_at.map(|dt| dt.to_rfc3339()),
            ),
        )?;
        Ok(())
    }

    /// Update a run's state and completion time.
    pub fn update_run(&self, run: &Run) -> Result<()> {
        self.conn.execute(
            "UPDATE runs SET state = ?1, completed_at = ?2 WHERE id = ?3",
            (
                serde_json::to_string(&run.state).unwrap(),
                run.completed_at.map(|dt| dt.to_rfc3339()),
                run.id.as_bytes().as_slice(),
            ),
        )?;
        Ok(())
    }

    /// Get a run by ID.
    pub fn get_run(&self, run_id: Uuid) -> Result<Option<Run>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, thread_id, prompt, state, worktree_id, created_at, completed_at
             FROM runs WHERE id = ?1",
        )?;

        let run = stmt
            .query_row([run_id.as_bytes().as_slice()], |row| {
                Ok(Run {
                    id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                    thread_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                    prompt: row.get(2)?,
                    state: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    worktree_id: row
                        .get_ref(4)?
                        .as_bytes_or_null()?
                        .map(|b| Uuid::from_slice(b).unwrap()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .into(),
                    completed_at: row
                        .get::<_, Option<String>>(6)?
                        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
                })
            })
            .optional()?;

        Ok(run)
    }

    /// List runs for a thread, ordered by creation time descending.
    pub fn list_runs(&self, thread_id: Uuid) -> Result<Vec<Run>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, thread_id, prompt, state, worktree_id, created_at, completed_at
             FROM runs WHERE thread_id = ?1 ORDER BY created_at DESC",
        )?;

        let runs = stmt
            .query_map([thread_id.as_bytes().as_slice()], |row| {
                Ok(Run {
                    id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                    thread_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                    prompt: row.get(2)?,
                    state: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                    worktree_id: row
                        .get_ref(4)?
                        .as_bytes_or_null()?
                        .map(|b| Uuid::from_slice(b).unwrap()),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .into(),
                    completed_at: row
                        .get::<_, Option<String>>(6)?
                        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(runs)
    }

    /// Insert a run event.
    pub fn insert_run_event(&self, event: &RunEvent) -> Result<()> {
        self.conn.execute(
            "INSERT INTO run_events (id, run_id, sequence, event_type, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                event.id.as_bytes().as_slice(),
                event.run_id.as_bytes().as_slice(),
                event.sequence,
                &event.event_type,
                serde_json::to_string(&event.payload).unwrap(),
                event.created_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    /// List run events for a run, ordered by sequence ascending.
    pub fn list_run_events(&self, run_id: Uuid) -> Result<Vec<RunEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, sequence, event_type, payload, created_at
             FROM run_events WHERE run_id = ?1 ORDER BY sequence ASC",
        )?;

        let events = stmt
            .query_map([run_id.as_bytes().as_slice()], |row| {
                Ok(RunEvent {
                    id: Uuid::from_slice(row.get_ref(0)?.as_bytes()?).unwrap(),
                    run_id: Uuid::from_slice(row.get_ref(1)?.as_bytes()?).unwrap(),
                    sequence: row.get(2)?,
                    event_type: row.get(3)?,
                    payload: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .into(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{Project, Thread, RunState};

    fn setup() -> (Database, Uuid) {
        let db = Database::in_memory().unwrap();

        // Create project and thread
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

        (db, thread.id)
    }

    #[test]
    fn inserts_and_retrieves_run() {
        let (db, thread_id) = setup();

        let run = Run {
            id: Uuid::new_v4(),
            thread_id,
            prompt: "Test prompt".into(),
            state: RunState::Queued,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };

        db.insert_run(&run).unwrap();
        let retrieved = db.get_run(run.id).unwrap().unwrap();

        assert_eq!(retrieved.id, run.id);
        assert_eq!(retrieved.prompt, "Test prompt");
        assert_eq!(retrieved.state, RunState::Queued);
    }

    #[test]
    fn updates_run_state() {
        let (db, thread_id) = setup();

        let mut run = Run {
            id: Uuid::new_v4(),
            thread_id,
            prompt: "Test".into(),
            state: RunState::Queued,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };

        db.insert_run(&run).unwrap();

        run.state = RunState::Completed;
        run.completed_at = Some(chrono::Utc::now());
        db.update_run(&run).unwrap();

        let retrieved = db.get_run(run.id).unwrap().unwrap();
        assert_eq!(retrieved.state, RunState::Completed);
        assert!(retrieved.completed_at.is_some());
    }

    #[test]
    fn lists_runs_for_thread() {
        let (db, thread_id) = setup();

        let run1 = Run {
            id: Uuid::new_v4(),
            thread_id,
            prompt: "First".into(),
            state: RunState::Completed,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };
        db.insert_run(&run1).unwrap();

        let run2 = Run {
            id: Uuid::new_v4(),
            thread_id,
            prompt: "Second".into(),
            state: RunState::Running,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        db.insert_run(&run2).unwrap();

        let runs = db.list_runs(thread_id).unwrap();
        assert_eq!(runs.len(), 2);
        // Most recent first
        assert_eq!(runs[0].prompt, "Second");
        assert_eq!(runs[1].prompt, "First");
    }

    #[test]
    fn inserts_and_lists_run_events() {
        let (db, thread_id) = setup();

        let run = Run {
            id: Uuid::new_v4(),
            thread_id,
            prompt: "Test".into(),
            state: RunState::Running,
            worktree_id: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        db.insert_run(&run).unwrap();

        let event1 = RunEvent {
            id: Uuid::new_v4(),
            run_id: run.id,
            sequence: 0,
            event_type: "RunStarted".into(),
            payload: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };
        db.insert_run_event(&event1).unwrap();

        let event2 = RunEvent {
            id: Uuid::new_v4(),
            run_id: run.id,
            sequence: 1,
            event_type: "TextDelta".into(),
            payload: serde_json::json!({"text": "Hello"}),
            created_at: chrono::Utc::now(),
        };
        db.insert_run_event(&event2).unwrap();

        let events = db.list_run_events(run.id).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "RunStarted");
        assert_eq!(events[1].event_type, "TextDelta");
    }
}
