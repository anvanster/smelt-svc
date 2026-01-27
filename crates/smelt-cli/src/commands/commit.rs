//! Commit command - commit with semantic delta

use anyhow::{Context, Result};
use chrono::Utc;
use smelt_core::{
    Author, AuthorType, ContextLinks, Git2Interface, GitInterface, ImpactSummary, IntentRecord,
    IntentStatus, SemanticDelta, SmeltError, SmeltGraph, SqliteStorage,
};
use smelt_validator::SmeltValidator;
use uuid::Uuid;

pub async fn run(
    intent_id: Option<String>,
    goal: Option<String>,
    skip_validation: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        return Err(SmeltError::NotInitialized.into());
    }

    let db_path = smelt_dir.join("smelt.db");
    let storage = SqliteStorage::open(&db_path).context("Failed to open database")?;

    let graph_path = smelt_dir.join("graph");
    let mut graph = SmeltGraph::open(&graph_path).context("Failed to open graph storage")?;

    let mut git = Git2Interface::open(&cwd)?;

    // Get or create intent
    let intent = if let Some(id) = intent_id {
        storage
            .find_intent_by_prefix(&id)?
            .ok_or_else(|| SmeltError::IntentNotFound(id))?
    } else if let Some(goal_text) = goal {
        // Create inline intent
        let (name, email) = git.get_author()?;
        let intent_id = Uuid::new_v4();

        // Capture baseline snapshot first
        let snapshot_id = graph.snapshot_for_intent(intent_id)?;

        let intent = IntentRecord {
            id: intent_id,
            created_at: Utc::now(),
            author: Author {
                name,
                email,
                author_type: AuthorType::Human,
            },
            goal: goal_text,
            rationale: None,
            constraints: Vec::new(),
            context_links: ContextLinks::default(),
            status: IntentStatus::InProgress,
            baseline_snapshot_id: Some(snapshot_id),
        };
        storage.store_intent(&intent)?;
        intent
    } else {
        // Check for active intent
        let intents = storage.list_intents(Some(IntentStatus::InProgress))?;
        if intents.is_empty() {
            eprintln!("No active intent found.");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  1. Create an intent first:  smelt intent create --goal \"Your goal\"");
            eprintln!("  2. Specify intent inline:   smelt commit --goal \"Your goal\"");
            eprintln!("  3. Use existing intent:     smelt commit --intent <id>");
            anyhow::bail!("No active intent");
        }
        intents.into_iter().next().unwrap()
    };

    println!(
        "Committing intent: {} ({})",
        &intent.id.to_string()[..8],
        intent.goal
    );

    // Get changed files
    let changed_files = git.changed_files()?;
    if changed_files.is_empty() {
        println!("No changes to commit.");
        println!();
        println!("Your working directory is clean. Make some code changes first,");
        println!("then run 'smelt commit' again.");
        return Ok(());
    }

    // Stage all changed files
    let staged_files = git.staged_files()?;
    if staged_files.is_empty() {
        git.stage_files(&changed_files)?;
        println!("  Staged {} files", changed_files.len());
    } else {
        println!("  {} files already staged", staged_files.len());
    }

    // Capture current snapshot
    let current_snapshot = graph.snapshot()?;

    // Compute semantic delta
    let delta = if let Some(baseline_id) = intent.baseline_snapshot_id {
        println!("  Computing semantic delta...");

        // TODO: Load baseline snapshot and compute full delta
        // For now, create placeholder delta with file-based summary
        let delta = SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            timestamp: Utc::now(),
            from_snapshot: baseline_id,
            to_snapshot: current_snapshot.id,
            changes: Vec::new(), // Will be populated when delta computation is complete
            impact_summary: ImpactSummary {
                files_affected: changed_files.len(),
                ..Default::default()
            },
        };
        Some(delta)
    } else {
        None
    };

    // Run validation
    if !skip_validation {
        println!("  Running validation...");

        let validator = SmeltValidator::from_smelt_dir(&smelt_dir);

        if let Some(ref delta) = delta {
            let outcome = validator.validate(delta, Some(&intent));

            // Display warnings
            for warning in outcome.warnings() {
                println!("    ⚠️  {} - {}", warning.rule, warning.message);
                if let Some(ref suggestion) = warning.suggestion {
                    println!("       → {}", suggestion);
                }
            }

            // Display errors and fail if any
            if outcome.has_errors() {
                for error in outcome.errors() {
                    eprintln!("    ❌ {} - {}", error.rule, error.message);
                    if let Some(ref location) = error.location {
                        eprintln!("       at {}", location);
                    }
                    if let Some(ref suggestion) = error.suggestion {
                        eprintln!("       → {}", suggestion);
                    }
                }
                anyhow::bail!(
                    "Validation failed: {} error(s), {} warning(s). Use --skip-validation to bypass.",
                    outcome.error_count,
                    outcome.warning_count
                );
            }

            if outcome.warning_count > 0 {
                println!("    Validation: passed with {} warning(s)", outcome.warning_count);
            } else {
                println!("    Validation: passed");
            }
        } else {
            println!("    Validation: skipped (no delta computed)");
        }
    } else {
        println!("  Skipping validation (--skip-validation)");
    }

    // Generate commit message
    let commit_message = generate_commit_message(&intent, delta.as_ref());

    // Create git commit
    println!("  Creating commit...");
    let git_sha = git.commit(&commit_message)?;

    // Store delta if computed
    if let Some(ref delta) = delta {
        storage.store_delta(delta)?;
    }

    // Update intent status
    storage.update_intent_status(
        intent.id,
        IntentStatus::Committed {
            git_sha: git_sha.clone(),
        },
    )?;

    // Store snapshot for this commit
    graph.store_snapshot(git_sha.clone(), current_snapshot);

    println!();
    println!(" Committed: {}", &git_sha[..8]);
    println!("   Intent: {}", &intent.id.to_string()[..8]);
    if let Some(ref delta) = delta {
        println!("   Delta:  {}", &delta.id.to_string()[..8]);
        println!("   Files:  {} changed", delta.impact_summary.files_affected);
    }

    Ok(())
}

fn generate_commit_message(intent: &IntentRecord, delta: Option<&SemanticDelta>) -> String {
    let mut message = intent.goal.clone();

    message.push_str("\n\n");
    message.push_str(&format!("Intent: {}\n", intent.id));

    if let Some(delta) = delta {
        message.push_str(&format!("Delta: {}\n", delta.id));

        let summary = &delta.impact_summary;
        let mut semantic_parts = Vec::new();

        if summary.functions_added > 0 {
            semantic_parts.push(format!("+{} functions", summary.functions_added));
        }
        if summary.functions_removed > 0 {
            semantic_parts.push(format!("-{} functions", summary.functions_removed));
        }
        if summary.functions_modified > 0 {
            semantic_parts.push(format!("~{} functions", summary.functions_modified));
        }
        if summary.breaking_changes > 0 {
            semantic_parts.push(format!("{} breaking", summary.breaking_changes));
        }

        if !semantic_parts.is_empty() {
            message.push_str(&format!("\nSemantic: {}", semantic_parts.join(", ")));
        }
    }

    if let Some(ref rationale) = intent.rationale {
        message.push_str(&format!("\nRationale: {}", rationale));
    }

    message
}
