//! Status command - show current semantic state

use anyhow::Result;
use smelt_core::{Git2Interface, GitInterface, IntentStatus, SmeltGraph, SqliteStorage};

pub async fn run(full: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        anyhow::bail!("Smelt not initialized. Run 'smelt init' first.");
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path)?;

    let graph_path = smelt_dir.join("graph");
    let graph = SmeltGraph::open(&graph_path)?;

    let git = Git2Interface::open(&cwd)?;

    // Find active intent
    let intents = storage.list_intents(Some(IntentStatus::InProgress))?;

    if let Some(intent) = intents.first() {
        println!(
            "Intent: {} ({})",
            &intent.id.to_string()[..8],
            intent.goal
        );
        println!("Status: In Progress");
        println!();

        // Get changed files (excluding .smelt/ directory)
        let changed_files: Vec<_> = git
            .changed_files()?
            .into_iter()
            .filter(|p| !p.starts_with(".smelt") && !p.to_string_lossy().contains("/.smelt/"))
            .collect();

        if changed_files.is_empty() {
            println!("No changes detected.");
        } else {
            println!("Changed files ({}):", changed_files.len());
            for file in &changed_files {
                let relative = file.strip_prefix(&cwd).unwrap_or(file);
                println!("  M {}", relative.display());
            }

            // Compute semantic delta if we have a baseline
            if let Some(_baseline_id) = intent.baseline_snapshot_id {
                println!();
                println!("Semantic changes:");

                // Capture current snapshot and compute delta
                let _current_snapshot = graph.snapshot()?;

                // TODO: Load baseline from storage and compute delta
                // For now, show placeholder based on file changes
                println!("  (Computing delta from {} files...)", changed_files.len());

                // Show summary
                println!();
                println!("Impact Summary:");
                println!("  Files affected: {}", changed_files.len());
            }
        }
    } else {
        println!("No active intent.");
        println!();

        // Show changed files anyway (excluding .smelt/ directory)
        let changed_files: Vec<_> = git
            .changed_files()?
            .into_iter()
            .filter(|p| !p.starts_with(".smelt") && !p.to_string_lossy().contains("/.smelt/"))
            .collect();
        if !changed_files.is_empty() {
            println!("Changed files ({}):", changed_files.len());
            for file in &changed_files {
                let relative = file.strip_prefix(&cwd).unwrap_or(file);
                println!("  M {}", relative.display());
            }
            println!();
            println!("Tip: Create an intent with 'smelt intent create --goal \"Your goal\"'");
            println!("     Or commit directly with 'smelt commit --goal \"Your goal\"'");
        } else {
            println!("No changes detected.");
        }
    }

    if full {
        println!();
        println!("Graph Statistics:");
        println!("  Nodes: {}", graph.node_count());
        println!("  Edges: {}", graph.edge_count());

        // Show recent intents
        let all_intents = storage.list_intents(None)?;
        if !all_intents.is_empty() {
            println!();
            println!("Recent Intents:");
            for intent in all_intents.iter().take(5) {
                let status = match &intent.status {
                    IntentStatus::Committed { .. } => "committed",
                    IntentStatus::InProgress => "in progress",
                    IntentStatus::Rejected { .. } => "rejected",
                    _ => "other",
                };
                println!("  {} - {} ({})", &intent.id.to_string()[..8], intent.goal, status);
            }
        }
    }

    Ok(())
}
