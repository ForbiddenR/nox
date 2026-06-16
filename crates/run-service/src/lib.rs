//! Run service: agent execution with real and mock runners.
//!
//! Supports two runner modes:
//! - Mock runner: emits fake events for testing (Step 8)
//! - Real runner: executes a local command/script and captures output (Step 9)

use chrono::Utc;
use models::{Run, RunEvent, RunState, Uuid};
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use storage::Database;

/// Errors from run operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("thread not found: {0}")]
    ThreadNotFound(Uuid),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("process error: {0}")]
    Process(#[from] std::io::Error),
}

/// Notification callback for run events.
pub type NotificationSender = Arc<dyn Fn(String, serde_json::Value) + Send + Sync>;

/// Start a new run with the mock runner (for testing).
pub fn start_run_mock(
    thread_id: Uuid,
    prompt: String,
    db: &Database,
    notification_tx: NotificationSender,
) -> Result<Uuid, Error> {
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

    let run_id = run.id;
    let db_path = db.path().map(|p| p.to_path_buf());

    std::thread::spawn(move || {
        if let Some(path) = db_path {
            if let Err(e) = emit_mock_events(run_id, &prompt, &path, notification_tx) {
                eprintln!("Mock runner error: {}", e);
            }
        }
    });

    Ok(run_id)
}

/// Start a new run with a real command execution.
///
/// The command receives the prompt via stdin and should output JSON events on stdout.
/// Each line should be a JSON object with: { "type": "text|tool_start|tool_end|error", "data": ... }
pub fn start_run_real(
    thread_id: Uuid,
    prompt: String,
    command: String,
    args: Vec<String>,
    db: &Database,
    notification_tx: NotificationSender,
) -> Result<Uuid, Error> {
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

    let run_id = run.id;
    let db_path = db.path().map(|p| p.to_path_buf());

    std::thread::spawn(move || {
        if let Some(path) = db_path {
            if let Err(e) = execute_real_command(run_id, &prompt, &command, args, &path, notification_tx) {
                eprintln!("Real runner error: {}", e);
            }
        }
    });

    Ok(run_id)
}

/// Execute a real command and stream its output as run events.
fn execute_real_command(
    run_id: Uuid,
    prompt: &str,
    command: &str,
    args: Vec<String>,
    db_path: &std::path::Path,
    tx: NotificationSender,
) -> Result<(), Error> {
    let db = Database::open(db_path).unwrap();
    let mut sequence = 0i64;

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

    emit("RunStarted", json!({ "run_id": run_id }))?;

    // Spawn the command
    let mut child = Command::new(command)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = writeln!(stdin, "{}", prompt);
    }

    // Read stdout line by line
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                // Try to parse as JSON event
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
                        let data = event.get("data").cloned().unwrap_or(json!({}));

                        match event_type {
                            "text" => emit("TextDelta", json!({ "text": data }))?,
                            "tool_start" => emit("ToolCallStarted", data)?,
                            "tool_end" => emit("ToolCallCompleted", data)?,
                            "error" => emit("Error", data)?,
                            _ => emit("TextDelta", json!({ "text": line }))?,
                        }
                    }
                } else {
                    // Not JSON, treat as plain text
                    emit("TextDelta", json!({ "text": line }))?;
                }
            }
        }
    }

    // Wait for completion
    let status = child.wait()?;

    // Update run state
    run.state = if status.success() {
        RunState::Completed
    } else {
        RunState::Failed
    };
    run.completed_at = Some(Utc::now());
    db.update_run(&run)?;

    if status.success() {
        emit("RunCompleted", json!({ "run_id": run_id, "exit_code": 0 }))?;
    } else {
        emit("RunFailed", json!({
            "run_id": run_id,
            "exit_code": status.code().unwrap_or(-1)
        }))?;
    }

    Ok(())
}

/// Emit a sequence of fake events for the mock run (Step 8 implementation).
fn emit_mock_events(
    run_id: Uuid,
    prompt: &str,
    db_path: &std::path::Path,
    tx: NotificationSender,
) -> Result<(), Error> {
    let db = Database::open(db_path).unwrap();
    let mut sequence = 0i64;

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

    let mut run = db.get_run(run_id)?.ok_or(Error::ThreadNotFound(run_id))?;
    run.state = RunState::Running;
    db.update_run(&run)?;

    emit("RunStarted", json!({ "run_id": run_id }))?;
    std::thread::sleep(std::time::Duration::from_millis(300));

    emit("TextDelta", json!({ "text": "Analyzing your request: " }))?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    emit("TextDelta", json!({ "text": &prompt }))?;
    std::thread::sleep(std::time::Duration::from_millis(400));

    emit("TextDelta", json!({ "text": "\n\nI'll help you with that. Let me start by examining the codebase..." }))?;
    std::thread::sleep(std::time::Duration::from_millis(500));

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

        let run_id = start_run_mock(thread_id, "Test prompt".into(), &db, tx).unwrap();

        let run = db.get_run(run_id).unwrap().unwrap();
        assert_eq!(run.prompt, "Test prompt");
        assert_eq!(run.thread_id, thread_id);
    }

    #[test]
    fn real_runner_executes_command() {
        use std::fs;

        // Create a temporary database file
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_run_{}.db", Uuid::new_v4()));

        let db = Database::open(&db_path).unwrap();

        // Set up project and thread
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

        let tx: NotificationSender = Arc::new(|_method, _params| {});

        // Use echo to simulate a simple agent that outputs JSON
        let run_id = start_run_real(
            thread.id,
            "test prompt".into(),
            "echo".into(),
            vec![r#"{"type":"text","data":"Hello from real runner"}"#.into()],
            &db,
            tx,
        )
        .unwrap();

        // Give it time to execute
        std::thread::sleep(std::time::Duration::from_millis(2000));

        let run = db.get_run(run_id).unwrap().unwrap();
        assert_eq!(run.prompt, "test prompt");
        assert_eq!(run.thread_id, thread.id);
        // Should be completed after wait
        assert!(
            matches!(run.state, RunState::Completed),
            "Expected Completed, got {:?}",
            run.state
        );

        // Check that events were created
        let events = db.list_run_events(run_id).unwrap();
        assert!(events.len() > 0, "Should have at least RunStarted event");

        // Clean up
        drop(db);
        let _ = fs::remove_file(db_path);
    }
}
