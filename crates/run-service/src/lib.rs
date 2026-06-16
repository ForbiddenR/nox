//! Mock run service: fake agent execution for UI testing.
//!
//! This mock runner creates a run, emits fake progress events (RunStarted, TextDelta,
//! ToolCallStarted, etc.), and completes. It will be replaced with a real runner in Step 9.

use chrono::Utc;
use models::{Run, RunEvent, RunState, Uuid};
use serde_json::json;
use std::sync::Arc;
use storage::Database;

/// Errors from run operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("thread not found: {0}")]
    ThreadNotFound(Uuid),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

/// Notification callback for run events.
pub type NotificationSender = Arc<dyn Fn(String, serde_json::Value) + Send + Sync>;

/// Start a new run in the given thread with a mock execution.
/// Returns the run ID immediately, then spawns a background task to emit events.
pub fn start_run(
    thread_id: Uuid,
    prompt: String,
    db: &Database,
    notification_tx: NotificationSender,
) -> Result<Uuid, Error> {
    // Create the run
    let run = Run {
        id: Uuid::new_v4(),
        thread_id,
        prompt: prompt.clone(),
        state: RunState::Queued,
        worktree_id: None,
        created_at: Utc::now(),
        completed_at: None,
    };

    db.insert_run(&run)?;

    // Spawn a background task to emit fake events
    let run_id = run.id;
    let db_path = db.path().map(|p| p.to_path_buf());

    std::thread::spawn(move || {
        // Only run mock events if we have a persistent database path
        if let Some(path) = db_path {
            if let Err(e) = emit_mock_events(run_id, &prompt, &path, notification_tx) {
                eprintln!("Mock runner error: {}", e);
            }
        }
    });

    Ok(run_id)
}

/// Emit a sequence of fake events for the mock run.
fn emit_mock_events(
    run_id: Uuid,
    prompt: &str,
    db_path: &std::path::Path,
    tx: NotificationSender,
) -> Result<(), Error> {
    let db = Database::open(db_path).unwrap();
    let mut sequence = 0i64;

    // Helper to emit and persist an event
    let mut emit = |event_type: &str, payload: serde_json::Value| -> Result<(), Error> {
        let event = RunEvent {
            id: Uuid::new_v4(),
            run_id,
            sequence,
            event_type: event_type.to_string(),
            payload: payload.clone(),
            created_at: Utc::now(),
        };
        db.insert_run_event(&event)?;

        // Notify frontend
        tx(
            "run:event".to_string(),
            json!({
                "run_id": run_id,
                "event": event
            }),
        );

        sequence += 1;
        Ok(())
    };

    // Update run state to Running
    let mut run = db.get_run(run_id)?.ok_or(Error::ThreadNotFound(run_id))?;
    run.state = RunState::Running;
    db.update_run(&run)?;

    // Emit RunStarted
    emit("RunStarted", json!({ "run_id": run_id }))?;
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Emit some text deltas simulating agent thinking
    emit("TextDelta", json!({ "text": "Analyzing your request: " }))?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    emit("TextDelta", json!({ "text": &prompt }))?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    emit("TextDelta", json!({ "text": "\n\nI'll help you with that. Let me start by examining the codebase..." }))?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Emit a fake tool call
    emit("ToolCallStarted", json!({
        "tool": "read_file",
        "args": { "path": "src/main.rs" }
    }))?;
    std::thread::sleep(std::time::Duration::from_millis(600));

    emit("ToolCallCompleted", json!({
        "tool": "read_file",
        "result": "File read successfully (100 lines)"
    }))?;
    std::thread::sleep(std::time::Duration::from_millis(300));

    emit("TextDelta", json!({ "text": "\n\nBased on my analysis, here's what I found..." }))?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    // Complete the run
    run.state = RunState::Completed;
    run.completed_at = Some(Utc::now());
    db.update_run(&run)?;

    emit("RunCompleted", json!({ "run_id": run_id }))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{Project, Thread};

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

        let thread = Thread {
            id: Uuid::new_v4(),
            project_id: project.id,
            title: "Test thread".into(),
            archived: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.threads().insert(&thread).unwrap();

        (db, thread.id)
    }

    #[test]
    fn creates_run() {
        let (db, thread_id) = setup();
        let tx: NotificationSender = Arc::new(|_method, _params| {});

        let run_id = start_run(thread_id, "Test prompt".into(), &db, tx).unwrap();

        let run = db.get_run(run_id).unwrap().unwrap();
        assert_eq!(run.prompt, "Test prompt");
        assert_eq!(run.thread_id, thread_id);
    }
}
