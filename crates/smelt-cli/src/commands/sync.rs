//! Sync command - recover from direct git commits

use anyhow::{Context, Result};
use smelt_core::{
    Author, AuthorType, ContextLinks, Git2Interface, GitInterface, IntentRecord, IntentStatus,
    SmeltError, SqliteStorage,
};
use uuid::Uuid;

/// Run the sync command
pub async fn run(dry_run: bool, limit: usize) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        return Err(SmeltError::NotInitialized.into());
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path).context("Failed to open database")?;

    let git = Git2Interface::open(&cwd).context("Failed to open git repository")?;

    println!("Scanning for untracked commits...");
    println!();

    // Get recent git commits
    let commits = git.list_commits(limit)?;

    if commits.is_empty() {
        println!("No commits found in repository.");
        return Ok(());
    }

    // Find commits not tracked by Smelt
    let mut untracked = Vec::new();

    for commit in &commits {
        if !storage.is_sha_tracked(&commit.sha)? {
            untracked.push(commit);
        }
    }

    if untracked.is_empty() {
        println!("All {} commits are tracked by Smelt.", commits.len());
        return Ok(());
    }

    println!(
        "Found {} untracked commits (out of {} scanned):",
        untracked.len(),
        commits.len()
    );
    println!();

    for (i, commit) in untracked.iter().enumerate() {
        println!(
            "  {}. [{}] {} ({})",
            i + 1,
            &commit.sha[..8],
            commit.summary,
            commit.timestamp.format("%Y-%m-%d %H:%M")
        );
        println!(
            "      Author: {} <{}>",
            commit.author_name, commit.author_email
        );
        if !commit.files_changed.is_empty() {
            let files_preview = if commit.files_changed.len() <= 3 {
                commit.files_changed.join(", ")
            } else {
                format!(
                    "{}, ... (+{} more)",
                    commit.files_changed[..3].join(", "),
                    commit.files_changed.len() - 3
                )
            };
            println!("      Files: {}", files_preview);
        }
        println!();
    }

    if dry_run {
        println!("Dry run - no changes made.");
        println!();
        println!("To create synthetic intents for these commits, run without --dry-run");
        return Ok(());
    }

    // Create synthetic intents for untracked commits
    println!("Creating synthetic intents...");
    println!();

    for commit in untracked {
        let intent_id = Uuid::new_v4();

        let intent = IntentRecord {
            id: intent_id,
            created_at: commit.timestamp,
            author: Author {
                name: commit.author_name.clone(),
                email: commit.author_email.clone(),
                author_type: AuthorType::Human,
            },
            goal: commit.summary.clone(),
            rationale: Some("Synthetic intent created by smelt sync".to_string()),
            constraints: Vec::new(),
            context_links: ContextLinks::default(),
            status: IntentStatus::InProgress, // Will be updated by store_synthetic_intent
            baseline_snapshot_id: None,
        };

        storage.store_synthetic_intent(&intent, &commit.sha)?;

        println!(
            "  Created intent {} for commit {}",
            &intent_id.to_string()[..8],
            &commit.sha[..8]
        );
    }

    println!();
    println!("Sync complete.");

    Ok(())
}

/// Show sync status
pub async fn status() -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        return Err(SmeltError::NotInitialized.into());
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path).context("Failed to open database")?;

    let git = Git2Interface::open(&cwd).context("Failed to open git repository")?;

    // Get recent git commits (last 100)
    let commits = git.list_commits(100)?;

    let mut tracked = 0;
    let mut untracked = 0;

    for commit in &commits {
        if storage.is_sha_tracked(&commit.sha)? {
            tracked += 1;
        } else {
            untracked += 1;
        }
    }

    println!("Sync Status");
    println!("-----------");
    println!("Commits scanned: {}", commits.len());
    println!("Tracked by Smelt: {}", tracked);
    println!("Untracked: {}", untracked);

    if untracked > 0 {
        println!();
        println!("Run 'smelt sync' to create synthetic intents for untracked commits.");
    }

    Ok(())
}
