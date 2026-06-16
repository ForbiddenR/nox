//! Git service: diff generation, status, and patch operations.

use std::path::Path;
use std::process::{Command, Stdio};

/// Errors from git operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("not a git repository: {0}")]
    NotGitRepo(String),
}

/// Summary of changes in a diff.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffSummary {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// A single changed file in the diff.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
    pub insertions: usize,
    pub deletions: usize,
}

/// File change status.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Git service for diff and patch operations.
pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    /// Get the working tree diff (unstaged + staged changes).
    pub fn get_working_tree_diff(&self, repo_path: &Path) -> Result<String, Error> {
        self.verify_git_repo(repo_path)?;

        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    /// List changed files with their status.
    pub fn list_changed_files(&self, repo_path: &Path) -> Result<Vec<ChangedFile>, Error> {
        self.verify_git_repo(repo_path)?;

        // Get file list with numstat
        let output = Command::new("git")
            .args(["diff", "HEAD", "--numstat"])
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut files = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let insertions = parts[0].parse().unwrap_or(0);
                let deletions = parts[1].parse().unwrap_or(0);
                let path = parts[2..].join(" ");

                let status = if insertions > 0 && deletions == 0 {
                    FileStatus::Added
                } else if insertions == 0 && deletions > 0 {
                    FileStatus::Deleted
                } else {
                    FileStatus::Modified
                };

                files.push(ChangedFile {
                    path,
                    status,
                    insertions,
                    deletions,
                });
            }
        }

        Ok(files)
    }

    /// Get diff summary statistics.
    pub fn get_diff_summary(&self, repo_path: &Path) -> Result<DiffSummary, Error> {
        let files = self.list_changed_files(repo_path)?;

        let files_changed = files.len();
        let insertions: usize = files.iter().map(|f| f.insertions).sum();
        let deletions: usize = files.iter().map(|f| f.deletions).sum();

        Ok(DiffSummary {
            files_changed,
            insertions,
            deletions,
        })
    }

    /// Generate a patch file content for current changes.
    pub fn generate_patch(&self, repo_path: &Path) -> Result<String, Error> {
        // Same as diff for now, but could add commit message header later
        self.get_working_tree_diff(repo_path)
    }

    /// Apply a patch to the working tree.
    pub fn apply_patch(&self, repo_path: &Path, patch: &str) -> Result<(), Error> {
        self.verify_git_repo(repo_path)?;

        let mut child = Command::new("git")
            .args(["apply", "--3way"])
            .current_dir(repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(patch.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Reset all changes in the working tree.
    pub fn reset_working_tree(&self, repo_path: &Path) -> Result<(), Error> {
        self.verify_git_repo(repo_path)?;

        let output = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // Also clean untracked files
        let output = Command::new("git")
            .args(["clean", "-fd"])
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(Error::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Verify that the path is a git repository.
    fn verify_git_repo(&self, repo_path: &Path) -> Result<(), Error> {
        if !repo_path.join(".git").exists() {
            return Err(Error::NotGitRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_service() {
        let _service = GitService::new();
    }

    #[test]
    fn detects_non_git_repo() {
        let service = GitService::new();
        let result = service.get_working_tree_diff(Path::new("/tmp"));
        assert!(result.is_err());
    }
}
