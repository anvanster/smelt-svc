// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Validate command - run validation without committing

use anyhow::Result;
use chrono::Utc;
use smelt_core::{
    Git2Interface, GitInterface, ImpactSummary, IntentStatus, SemanticDelta, SmeltGraph,
    SqliteStorage,
};
use smelt_validator::SmeltValidator;
use uuid::Uuid;

pub async fn run(intent_id: Option<String>, strict: bool, show_config: bool) -> Result<()> {
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

    // Create validator
    let validator = if strict {
        SmeltValidator::strict()
    } else {
        SmeltValidator::from_smelt_dir(&smelt_dir)
    };

    if show_config {
        println!("Validation configuration:");
        println!("{:#?}", validator.config());
        return Ok(());
    }

    // Find intent if specified
    let intent = if let Some(id) = intent_id {
        Some(
            storage
                .find_intent_by_prefix(&id)?
                .ok_or_else(|| anyhow::anyhow!("Intent not found: {}", id))?,
        )
    } else {
        // Try to find active intent
        storage
            .list_intents(Some(IntentStatus::InProgress))?
            .into_iter()
            .next()
    };

    // Get changed files
    let changed_files = git.changed_files()?;
    if changed_files.is_empty() {
        println!("No changes to validate.");
        return Ok(());
    }

    println!("Validating {} changed files...", changed_files.len());

    // Capture current snapshot
    let current_snapshot = graph.snapshot()?;

    // Compute delta (simplified - using file count for now)
    let baseline_id = intent
        .as_ref()
        .and_then(|i| i.baseline_snapshot_id)
        .unwrap_or(current_snapshot.id);

    let delta = SemanticDelta {
        id: Uuid::new_v4(),
        intent_id: intent.as_ref().map(|i| i.id).unwrap_or(Uuid::new_v4()),
        timestamp: Utc::now(),
        from_snapshot: baseline_id,
        to_snapshot: current_snapshot.id,
        changes: Vec::new(),
        impact_summary: ImpactSummary {
            files_affected: changed_files.len(),
            ..Default::default()
        },
    };

    // Run validation
    let outcome = validator.validate(&delta, intent.as_ref());

    // Display results
    println!();
    println!("Validation Results:");
    println!("==================");

    if outcome.info_count > 0 {
        println!();
        println!("Info ({}):", outcome.info_count);
        for violation in outcome
            .violations
            .iter()
            .filter(|v| v.severity == smelt_validator::ValidationSeverity::Info)
        {
            println!("  ℹ️  {} - {}", violation.rule, violation.message);
        }
    }

    if outcome.warning_count > 0 {
        println!();
        println!("Warnings ({}):", outcome.warning_count);
        for warning in outcome.warnings() {
            println!("  ⚠️  {} - {}", warning.rule, warning.message);
            if let Some(ref location) = warning.location {
                println!("     at {}", location);
            }
            if let Some(ref suggestion) = warning.suggestion {
                println!("     → {}", suggestion);
            }
        }
    }

    if outcome.error_count > 0 {
        println!();
        println!("Errors ({}):", outcome.error_count);
        for error in outcome.errors() {
            eprintln!("  ❌ {} - {}", error.rule, error.message);
            if let Some(ref location) = error.location {
                eprintln!("     at {}", location);
            }
            if let Some(ref suggestion) = error.suggestion {
                eprintln!("     → {}", suggestion);
            }
        }
    }

    println!();
    if outcome.passed {
        println!("✅ Validation passed");
        if outcome.warning_count > 0 {
            println!(
                "   {} warning(s) - consider addressing before commit",
                outcome.warning_count
            );
        }
    } else {
        println!("❌ Validation failed: {} error(s)", outcome.error_count);
        std::process::exit(1);
    }

    Ok(())
}
