//! Project service: open projects, detect Git repos, gather metadata.

use std::path::Path;

use chrono::Utc;
use models::{Project, RepoStatusSummary, Uuid};
use storage::ProjectStore;

/// Errors from project operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("project path does not exist: {0}")]
    PathNotFound(String),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("git error: {0}")]
    Git(String),
}

/// Open a project at the given path, gather metadata, and persist it.
pub fn open_project(path: &str, store: &ProjectStore) -> Result<Project, Error> {
    let path_buf = std::path::PathBuf::from(path);
    if !path_buf.exists() {
        return Err(Error::PathNotFound(path.to_string()));
    }

    let canonical = path_buf
        .canonicalize()
        .map_err(|_| Error::PathNotFound(path.to_string()))?;
    let path_str = canonical.to_string_lossy().to_string();

    // Check if it's a Git repo.
    let is_git_repo = canonical.join(".git").exists();
    let current_branch = if is_git_repo {
        get_current_branch(&canonical).ok()
    } else {
        None
    };

    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string();

    let now = Utc::now();

    // Check if project already exists by path; reuse its ID if so.
    let project = if let Some(existing) = store.get_by_path(&path_str)? {
        Project {
            id: existing.id,
            name,
            path: path_str,
            is_git_repo,
            current_branch,
            created_at: existing.created_at,
            last_opened_at: now,
        }
    } else {
        Project {
            id: Uuid::new_v4(),
            name,
            path: path_str,
            is_git_repo,
            current_branch,
            created_at: now,
            last_opened_at: now,
        }
    };

    store.upsert(&project)?;
    Ok(project)
}

/// Get the current branch name from a Git repo.
fn get_current_branch(repo_path: &Path) -> Result<String, Error> {
    let head_path = repo_path.join(".git/HEAD");
    let content = std::fs::read_to_string(&head_path)
        .map_err(|e| Error::Git(format!("failed to read HEAD: {e}")))?;

    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        Ok(branch.trim().to_string())
    } else {
        // Detached HEAD or other state.
        Ok("(detached)".to_string())
    }
}

/// Get a brief status summary for a Git repo (placeholder implementation).
pub fn get_repo_status(_repo_path: &Path) -> Result<RepoStatusSummary, Error> {
    // For Milestone 1, return a stub. Full implementation will use libgit2 or
    // shell out to `git status --porcelain`.
    Ok(RepoStatusSummary {
        branch: "main".to_string(),
        dirty: false,
        ahead: 0,
        behind: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::Database;

    #[test]
    fn opens_existing_directory() {
        let db = Database::in_memory().unwrap();
        let store = db.projects();
        let result = open_project("/tmp", &store);
        assert!(result.is_ok());
        let project = result.unwrap();
        assert_eq!(project.name, "tmp");
    }

    #[test]
    fn rejects_nonexistent_path() {
        let db = Database::in_memory().unwrap();
        let store = db.projects();
        let result = open_project("/this/does/not/exist/hopefully", &store);
        assert!(matches!(result, Err(Error::PathNotFound(_))));
    }
}
