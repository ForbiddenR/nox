//! Shared domain types and DTOs for the sidecar services.
//!
//! These models are exchanged between the Electron main process and the Rust
//! sidecar over JSON-RPC, and are also persisted in SQLite.

use serde::{Deserialize, Serialize};

// Re-export common types used throughout the codebase.
pub use chrono::{DateTime, Utc};
pub use uuid::Uuid;

/// A local project (typically a Git repo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub is_git_repo: bool,
    pub current_branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: DateTime<Utc>,
}

/// Brief summary of a project's Git status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatusSummary {
    pub branch: String,
    pub dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}

/// A conversation thread within a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single agent run within a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub prompt: String,
    pub state: RunState,
    pub worktree_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Run lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Queued,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

/// A timeline event within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub id: Uuid,
    pub run_id: Uuid,
    pub sequence: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A Git worktree for isolated run execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub path: String,
    pub branch: String,
    pub head_sha: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// A terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub worktree_id: Option<Uuid>,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// A stored artifact (diff, patch, log, summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub run_id: Uuid,
    pub artifact_type: ArtifactType,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Diff,
    Patch,
    Log,
    Summary,
}
