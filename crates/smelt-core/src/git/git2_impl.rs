//! Git2-rs implementation of GitInterface

use crate::{Result, SmeltError};
use chrono::{TimeZone, Utc};
use git2::{DiffOptions, Repository, Signature, StatusOptions};
use std::path::PathBuf;

use super::{CommitInfo, GitInterface};

/// Git interface implementation using git2-rs
pub struct Git2Interface {
    repo: Repository,
    root: PathBuf,
}

impl Git2Interface {
    /// Open a git repository at the given path
    pub fn open(path: &PathBuf) -> Result<Self> {
        let repo = Repository::discover(path)?;
        let root = repo
            .workdir()
            .ok_or_else(|| SmeltError::Git("Bare repository not supported".into()))?
            .to_path_buf();

        Ok(Self { repo, root })
    }

    /// Get the git2 repository reference
    pub fn repo(&self) -> &Repository {
        &self.repo
    }
}

impl GitInterface for Git2Interface {
    fn is_initialized(&self) -> bool {
        !self.repo.is_empty().unwrap_or(true)
    }

    fn head_sha(&self) -> Result<String> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        if head.is_branch() {
            head.shorthand()
                .map(|s| s.to_string())
                .ok_or_else(|| SmeltError::Git("Invalid branch name".into()))
        } else {
            // Detached HEAD
            let commit = head.peel_to_commit()?;
            Ok(format!(
                "HEAD detached at {}",
                &commit.id().to_string()[..8]
            ))
        }
    }

    fn get_author(&self) -> Result<(String, String)> {
        let config = self.repo.config()?;

        let name = config
            .get_string("user.name")
            .map_err(|_| SmeltError::GitAuthorNotConfigured)?;

        let email = config
            .get_string("user.email")
            .map_err(|_| SmeltError::GitAuthorNotConfigured)?;

        Ok((name, email))
    }

    fn changed_files(&self) -> Result<Vec<PathBuf>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut files = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                // Include modified, new, deleted, renamed files
                if status.is_wt_modified()
                    || status.is_wt_new()
                    || status.is_wt_deleted()
                    || status.is_wt_renamed()
                    || status.is_index_modified()
                    || status.is_index_new()
                    || status.is_index_deleted()
                    || status.is_index_renamed()
                {
                    files.push(self.root.join(path));
                }
            }
        }

        Ok(files)
    }

    fn staged_files(&self) -> Result<Vec<PathBuf>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(false).include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut files = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                // Only include files staged in the index
                if status.is_index_modified()
                    || status.is_index_new()
                    || status.is_index_deleted()
                    || status.is_index_renamed()
                {
                    files.push(self.root.join(path));
                }
            }
        }

        Ok(files)
    }

    fn stage_files(&mut self, files: &[PathBuf]) -> Result<()> {
        let mut index = self.repo.index()?;

        // Canonicalize repo root for proper comparison
        let canonical_root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());

        for file in files {
            // Canonicalize the file path if it exists
            let canonical_file = if file.exists() {
                file.canonicalize().unwrap_or_else(|_| file.clone())
            } else {
                file.clone()
            };

            // Get path relative to repo root
            let relative = canonical_file
                .strip_prefix(&canonical_root)
                .or_else(|_| file.strip_prefix(&self.root))
                .unwrap_or(file);

            if file.exists() {
                index.add_path(relative)?;
            } else {
                // File was deleted
                index.remove_path(relative)?;
            }
        }

        index.write()?;
        Ok(())
    }

    fn commit(&mut self, message: &str) -> Result<String> {
        let (name, email) = self.get_author()?;
        let signature = Signature::now(&name, &email)?;

        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let parent_commit = if let Ok(head) = self.repo.head() {
            Some(head.peel_to_commit()?)
        } else {
            None
        };

        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

        let commit_id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        Ok(commit_id.to_string())
    }

    fn has_uncommitted_changes(&self) -> Result<bool> {
        let changed = self.changed_files()?;
        Ok(!changed.is_empty())
    }

    fn repo_root(&self) -> &PathBuf {
        &self.root
    }

    fn list_commits(&self, limit: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();

        for oid in revwalk.take(limit) {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let timestamp = Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .unwrap_or_else(Utc::now);

            let author = commit.author();

            // Get files changed
            let files_changed = self.get_commit_files(&commit)?;

            let message = commit.message().unwrap_or("").to_string();
            let summary = message.lines().next().unwrap_or("").to_string();

            commits.push(CommitInfo {
                sha: oid.to_string(),
                summary,
                message,
                author_name: author.name().unwrap_or("Unknown").to_string(),
                author_email: author.email().unwrap_or("").to_string(),
                timestamp,
                files_changed,
            });
        }

        Ok(commits)
    }

    fn get_commit(&self, sha: &str) -> Result<Option<CommitInfo>> {
        let oid = match git2::Oid::from_str(sha) {
            Ok(oid) => oid,
            Err(_) => return Ok(None),
        };

        let commit = match self.repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let timestamp = Utc
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now);

        let author = commit.author();
        let files_changed = self.get_commit_files(&commit)?;

        let message = commit.message().unwrap_or("").to_string();
        let summary = message.lines().next().unwrap_or("").to_string();

        Ok(Some(CommitInfo {
            sha: oid.to_string(),
            summary,
            message,
            author_name: author.name().unwrap_or("Unknown").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            timestamp,
            files_changed,
        }))
    }

    fn diff_commits(&self, from_sha: &str, to_sha: &str) -> Result<Vec<String>> {
        let from_oid = git2::Oid::from_str(from_sha)?;
        let to_oid = git2::Oid::from_str(to_sha)?;

        let from_commit = self.repo.find_commit(from_oid)?;
        let to_commit = self.repo.find_commit(to_oid)?;

        let from_tree = from_commit.tree()?;
        let to_tree = to_commit.tree()?;

        let mut opts = DiffOptions::new();
        let diff =
            self.repo
                .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut opts))?;

        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                } else if let Some(path) = delta.old_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok(files)
    }
}

impl Git2Interface {
    /// Get files changed in a commit
    fn get_commit_files(&self, commit: &git2::Commit) -> Result<Vec<String>> {
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut opts = DiffOptions::new();
        let diff =
            self.repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                } else if let Some(path) = delta.old_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;

        Ok(files)
    }
}
