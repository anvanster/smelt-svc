// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Intent management commands

use anyhow::Result;
use chrono::Utc;
use smelt_core::{
    Author, AuthorType, ContextLinks, Git2Interface, GitInterface, IntentRecord, IntentStatus,
    SmeltGraph, SqliteStorage,
};
use uuid::Uuid;

pub async fn create(goal: String, rationale: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        anyhow::bail!("Smelt not initialized. Run 'smelt init' first.");
    }

    // Get author info from git
    let git = Git2Interface::open(&cwd)?;
    let (name, email) = git.get_author()?;

    // Create intent record
    let intent_id = Uuid::new_v4();
    let mut intent = IntentRecord {
        id: intent_id,
        created_at: Utc::now(),
        author: Author {
            name,
            email,
            author_type: AuthorType::Human,
        },
        goal: goal.clone(),
        rationale,
        constraints: Vec::new(),
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: None,
    };

    // Capture baseline snapshot
    let graph_path = smelt_dir.join("graph");
    let mut graph = SmeltGraph::open(&graph_path)?;
    let snapshot_id = graph.snapshot_for_intent(intent_id)?;

    // Update intent with snapshot
    intent.baseline_snapshot_id = Some(snapshot_id);

    // Store intent
    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path)?;
    storage.store_intent(&intent)?;

    println!("Created intent: {}", short_id(intent.id));
    println!();
    println!("  Goal: {}", goal);
    println!("  Status: In Progress");
    println!("  Baseline snapshot: {}", short_id(snapshot_id));
    println!();
    println!("Now make your code changes, then run 'smelt status' to see semantic changes.");

    Ok(())
}

pub async fn list(status_filter: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        anyhow::bail!("Smelt not initialized. Run 'smelt init' first.");
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path)?;

    let status = status_filter.map(|s| parse_status(&s)).transpose()?;
    let intents = storage.list_intents(status)?;

    if intents.is_empty() {
        println!("No intents found.");
        return Ok(());
    }

    println!("Intents:");
    println!();
    for intent in intents {
        let status_icon = match &intent.status {
            IntentStatus::Draft => " ",
            IntentStatus::InProgress => " ",
            IntentStatus::PendingValidation => " ",
            IntentStatus::Validated => " ",
            IntentStatus::Committed { .. } => " ",
            IntentStatus::Rejected { .. } => " ",
            IntentStatus::Abandoned => " ",
        };

        println!(
            "  {} {} - {}",
            status_icon,
            short_id(intent.id),
            intent.goal
        );
        println!(
            "      Created: {} | Status: {}",
            intent.created_at.format("%Y-%m-%d %H:%M"),
            format_status(&intent.status)
        );
    }

    Ok(())
}

pub async fn show(id: String) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        anyhow::bail!("Smelt not initialized. Run 'smelt init' first.");
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path)?;

    // Support partial ID matching
    let intent = storage
        .find_intent_by_prefix(&id)?
        .ok_or_else(|| anyhow::anyhow!("Intent not found: {}", id))?;

    println!("Intent: {}", intent.id);
    println!();
    println!("  Goal:      {}", intent.goal);
    if let Some(ref rationale) = intent.rationale {
        println!("  Rationale: {}", rationale);
    }
    println!(
        "  Author:    {} <{}>",
        intent.author.name, intent.author.email
    );
    println!(
        "  Created:   {}",
        intent.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("  Status:    {}", format_status(&intent.status));

    if let Some(snapshot_id) = intent.baseline_snapshot_id {
        println!("  Baseline:  {}", snapshot_id);
    }

    if !intent.constraints.is_empty() {
        println!("\n  Constraints:");
        for c in &intent.constraints {
            println!("    - {}: {}", c.name, c.value);
        }
    }

    Ok(())
}

fn parse_status(s: &str) -> Result<IntentStatus> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(IntentStatus::Draft),
        "in_progress" | "inprogress" | "in-progress" => Ok(IntentStatus::InProgress),
        "pending" | "pending_validation" => Ok(IntentStatus::PendingValidation),
        "validated" => Ok(IntentStatus::Validated),
        "committed" => Ok(IntentStatus::Committed {
            git_sha: String::new(),
        }),
        "rejected" => Ok(IntentStatus::Rejected {
            violations: Vec::new(),
        }),
        "abandoned" => Ok(IntentStatus::Abandoned),
        _ => anyhow::bail!("Unknown status: {}", s),
    }
}

fn format_status(status: &IntentStatus) -> String {
    match status {
        IntentStatus::Draft => "Draft".to_string(),
        IntentStatus::InProgress => "In Progress".to_string(),
        IntentStatus::PendingValidation => "Pending Validation".to_string(),
        IntentStatus::Validated => "Validated".to_string(),
        IntentStatus::Committed { git_sha } => {
            format!("Committed ({})", &git_sha[..8.min(git_sha.len())])
        }
        IntentStatus::Rejected { violations } => {
            format!("Rejected ({} violations)", violations.len())
        }
        IntentStatus::Abandoned => "Abandoned".to_string(),
    }
}

fn short_id(id: Uuid) -> String {
    id.to_string()[..8].to_string()
}
