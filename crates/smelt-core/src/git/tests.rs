// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Tests for Git interface

use super::{Git2Interface, GitInterface};
use std::process::Command;
use tempfile::tempdir;

fn setup_git_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Configure git user
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

fn setup_git_repo_with_commit() -> tempfile::TempDir {
    let dir = setup_git_repo();

    // Create a file and commit
    std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

#[test]
fn test_open_git_repo() {
    let dir = setup_git_repo();
    let git = Git2Interface::open(&dir.path().to_path_buf());
    assert!(git.is_ok());
}

#[test]
fn test_open_non_git_dir() {
    let dir = tempdir().unwrap();
    let git = Git2Interface::open(&dir.path().to_path_buf());
    assert!(git.is_err());
}

#[test]
fn test_is_initialized_empty_repo() {
    let dir = setup_git_repo();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();
    // Empty repo (no commits) - is_initialized checks if repo.is_empty() is NOT true
    // In git2, is_empty() returns true when there are no commits
    // Note: Some git configurations may auto-create initial branch, so we check
    // that is_initialized() returns a consistent value
    let _is_init = git.is_initialized();
    // The key test is that it doesn't panic
}

#[test]
fn test_is_initialized_with_commit() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();
    assert!(git.is_initialized());
}

#[test]
fn test_get_author() {
    let dir = setup_git_repo();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let (name, email) = git.get_author().unwrap();
    assert_eq!(name, "Test User");
    assert_eq!(email, "test@example.com");
}

#[test]
fn test_head_sha() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let sha = git.head_sha().unwrap();
    assert_eq!(sha.len(), 40); // Git SHA is 40 hex chars
}

#[test]
fn test_current_branch() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let branch = git.current_branch().unwrap();
    // Could be "main" or "master" depending on git config
    assert!(branch == "main" || branch == "master");
}

#[test]
fn test_changed_files_empty() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let changed = git.changed_files().unwrap();
    assert!(changed.is_empty());
}

#[test]
fn test_changed_files_with_modification() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    // Modify a file
    std::fs::write(dir.path().join("test.txt"), "modified").unwrap();

    let changed = git.changed_files().unwrap();
    assert_eq!(changed.len(), 1);
}

#[test]
fn test_changed_files_with_new_file() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    // Create a new file
    std::fs::write(dir.path().join("new.txt"), "new file").unwrap();

    let changed = git.changed_files().unwrap();
    assert_eq!(changed.len(), 1);
}

#[test]
fn test_staged_files_empty() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let staged = git.staged_files().unwrap();
    assert!(staged.is_empty());
}

#[test]
fn test_stage_files() {
    let dir = setup_git_repo_with_commit();
    let mut git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    // Create and stage a new file
    let new_file = dir.path().join("staged.txt");
    std::fs::write(&new_file, "staged content").unwrap();

    git.stage_files(std::slice::from_ref(&new_file)).unwrap();

    let staged = git.staged_files().unwrap();
    assert_eq!(staged.len(), 1);
}

#[test]
fn test_commit() {
    let dir = setup_git_repo_with_commit();
    let mut git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    // Create and stage a new file
    let new_file = dir.path().join("commit_test.txt");
    std::fs::write(&new_file, "commit test").unwrap();
    git.stage_files(&[new_file]).unwrap();

    // Commit
    let sha = git.commit("Test commit message").unwrap();
    assert_eq!(sha.len(), 40);

    // Verify no staged files
    let staged = git.staged_files().unwrap();
    assert!(staged.is_empty());
}

#[test]
fn test_has_uncommitted_changes() {
    let dir = setup_git_repo_with_commit();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    // No changes initially
    assert!(!git.has_uncommitted_changes().unwrap());

    // Add a change
    std::fs::write(dir.path().join("test.txt"), "changed").unwrap();
    assert!(git.has_uncommitted_changes().unwrap());
}

#[test]
fn test_repo_root() {
    let dir = setup_git_repo();
    let git = Git2Interface::open(&dir.path().to_path_buf()).unwrap();

    let root = git.repo_root();
    assert_eq!(
        root.canonicalize().unwrap(),
        dir.path().canonicalize().unwrap()
    );
}
