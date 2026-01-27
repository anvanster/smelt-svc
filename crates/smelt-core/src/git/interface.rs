//! Git interface trait for abstraction over git implementations

use crate::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Information about a git commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Commit SHA
    pub sha: String,
    /// Commit message (first line)
    pub summary: String,
    /// Full commit message
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
    /// Files changed in this commit
    pub files_changed: Vec<String>,
}

/// Abstraction over git operations
pub trait GitInterface {
    /// Check if the repository is initialized
    fn is_initialized(&self) -> bool;

    /// Get the current HEAD commit SHA
    fn head_sha(&self) -> Result<String>;

    /// Get the current branch name
    fn current_branch(&self) -> Result<String>;

    /// Get author info from git config (name, email)
    fn get_author(&self) -> Result<(String, String)>;

    /// Get list of changed files (modified, added, deleted)
    fn changed_files(&self) -> Result<Vec<PathBuf>>;

    /// Get list of staged files
    fn staged_files(&self) -> Result<Vec<PathBuf>>;

    /// Stage files for commit
    fn stage_files(&mut self, files: &[PathBuf]) -> Result<()>;

    /// Create a commit with the given message
    fn commit(&mut self, message: &str) -> Result<String>;

    /// Check if there are uncommitted changes
    fn has_uncommitted_changes(&self) -> Result<bool>;

    /// Get the repository root path
    fn repo_root(&self) -> &PathBuf;

    /// List recent commits (most recent first)
    fn list_commits(&self, limit: usize) -> Result<Vec<CommitInfo>>;

    /// Get commit info by SHA
    fn get_commit(&self, sha: &str) -> Result<Option<CommitInfo>>;

    /// Get files changed between two commits
    fn diff_commits(&self, from_sha: &str, to_sha: &str) -> Result<Vec<String>>;
}
