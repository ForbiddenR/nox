//! Git worktree management service.
//!
//! Provides operations for creating, listing, switching, and removing Git worktrees
//! for isolated development contexts.

use anyhow::{anyhow, Result};
use chrono::Utc;
use models::Worktree;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

/// Service for managing Git worktrees.
pub struct WorktreeService {
    worktrees_base: PathBuf,
}

impl WorktreeService {
    /// Create a new worktree service.
    ///
    /// # Arguments
    /// * `worktrees_base` - Base directory where worktrees will be created (e.g., `.cox/worktrees`)
    pub fn new(worktrees_base: PathBuf) -> Self {
        Self { worktrees_base }
    }

    /// List all worktrees for a repository.
    pub fn list_worktrees(&self, repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
        self.verify_git_repo(repo_path)?;

        let output = Command::new("git")
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git worktree list failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_worktree_list(&stdout)
    }

    /// Create a new worktree.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the main repository
    /// * `project_id` - ID of the project this worktree belongs to
    /// * `name` - Name for the worktree (used in path)
    /// * `base_ref` - Git ref to branch from (e.g., "HEAD", "main", "origin/main")
    pub fn create_worktree(
        &self,
        repo_path: &Path,
        project_id: Uuid,
        name: &str,
        base_ref: &str,
    ) -> Result<Worktree> {
        self.verify_git_repo(repo_path)?;

        // Create worktrees base directory if it doesn't exist
        std::fs::create_dir_all(&self.worktrees_base)?;

        // Generate unique branch name
        let branch_name = format!("cox/{}", name);
        let worktree_path = self.worktrees_base.join(name);

        // Ensure path doesn't already exist
        if worktree_path.exists() {
            return Err(anyhow!("Worktree path already exists: {:?}", worktree_path));
        }

        // Create the worktree
        let output = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(&branch_name)
            .arg(&worktree_path)
            .arg(base_ref)
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git worktree add failed: {}", stderr));
        }

        // Get HEAD SHA
        let head_sha = self.get_head_sha(&worktree_path)?;

        Ok(Worktree {
            id: Uuid::new_v4(),
            project_id,
            name: name.to_string(),
            path: worktree_path.to_string_lossy().to_string(),
            branch: branch_name,
            head_sha,
            is_active: false,
            created_at: Utc::now(),
        })
    }

    /// Remove a worktree.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the main repository
    /// * `worktree_path` - Path to the worktree to remove
    /// * `force` - Force removal even if worktree has uncommitted changes
    pub fn remove_worktree(
        &self,
        repo_path: &Path,
        worktree_path: &Path,
        force: bool,
    ) -> Result<()> {
        self.verify_git_repo(repo_path)?;

        let mut cmd = Command::new("git");
        cmd.arg("worktree")
            .arg("remove")
            .arg(worktree_path)
            .current_dir(repo_path);

        if force {
            cmd.arg("--force");
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git worktree remove failed: {}", stderr));
        }

        Ok(())
    }

    /// Get the current HEAD SHA of a worktree.
    fn get_head_sha(&self, worktree_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git rev-parse HEAD failed: {}", stderr));
        }

        let sha = String::from_utf8_lossy(&output.stdout);
        Ok(sha.trim().to_string())
    }

    /// Verify that a path is a Git repository.
    fn verify_git_repo(&self, path: &Path) -> Result<()> {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow!("Not a git repository: {:?}", path));
        }
        Ok(())
    }

    /// Parse the output of `git worktree list --porcelain`.
    fn parse_worktree_list(&self, output: &str) -> Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();
        let mut current = WorktreeInfo::default();

        for line in output.lines() {
            if line.is_empty() {
                if !current.path.is_empty() {
                    worktrees.push(current.clone());
                    current = WorktreeInfo::default();
                }
                continue;
            }

            if let Some(path) = line.strip_prefix("worktree ") {
                current.path = path.to_string();
            } else if let Some(sha) = line.strip_prefix("HEAD ") {
                current.head_sha = sha.to_string();
            } else if let Some(branch) = line.strip_prefix("branch ") {
                current.branch = Some(branch.to_string());
            } else if line == "bare" {
                current.is_bare = true;
            } else if line == "detached" {
                current.is_detached = true;
            }
        }

        // Push the last worktree if any
        if !current.path.is_empty() {
            worktrees.push(current);
        }

        Ok(worktrees)
    }
}

/// Information about a worktree returned by Git.
#[derive(Debug, Clone, Default)]
pub struct WorktreeInfo {
    pub path: String,
    pub head_sha: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_detached: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_test_repo(temp_dir: &TempDir) -> PathBuf {
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_path).unwrap();

        // Initialize git repo
        Command::new("git")
            .arg("init")
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Configure git
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create initial commit
        fs::write(repo_path.join("README.md"), "# Test\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        repo_path
    }

    #[test]
    fn test_create_and_list_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = init_test_repo(&temp_dir);
        let worktrees_base = temp_dir.path().join("worktrees");

        let service = WorktreeService::new(worktrees_base.clone());

        // Create a worktree
        let project_id = Uuid::new_v4();
        let result = service.create_worktree(&repo_path, project_id, "feature-1", "HEAD");
        assert!(result.is_ok());

        let worktree = result.unwrap();
        assert_eq!(worktree.project_id, project_id);
        assert_eq!(worktree.name, "feature-1");
        assert_eq!(worktree.branch, "cox/feature-1");
        assert!(!worktree.head_sha.is_empty());

        // List worktrees
        let worktrees = service.list_worktrees(&repo_path).unwrap();
        assert!(worktrees.len() >= 2); // Main + our new worktree

        // Verify our worktree appears in the list
        let found = worktrees
            .iter()
            .any(|wt| wt.path.contains("feature-1"));
        assert!(found);
    }

    #[test]
    fn test_remove_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = init_test_repo(&temp_dir);
        let worktrees_base = temp_dir.path().join("worktrees");

        let service = WorktreeService::new(worktrees_base.clone());

        // Create a worktree
        let project_id = Uuid::new_v4();
        let worktree = service
            .create_worktree(&repo_path, project_id, "feature-2", "HEAD")
            .unwrap();

        let worktree_path = Path::new(&worktree.path);
        assert!(worktree_path.exists());

        // Remove it
        let result = service.remove_worktree(&repo_path, worktree_path, false);
        assert!(result.is_ok());

        // Verify it's gone
        assert!(!worktree_path.exists());
    }

    #[test]
    fn test_verify_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let non_repo = temp_dir.path().join("not-a-repo");
        fs::create_dir(&non_repo).unwrap();

        let service = WorktreeService::new(temp_dir.path().join("worktrees"));

        let result = service.verify_git_repo(&non_repo);
        assert!(result.is_err());
    }
}
