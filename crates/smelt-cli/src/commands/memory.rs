//! Memory command - manage episodic memory

use anyhow::Result;
use smelt_memory::{Episode, EpisodeOutcome, SmeltMemory};
use uuid::Uuid;

/// Get the memory system for the current project
fn get_memory() -> Result<SmeltMemory> {
    let cwd = std::env::current_dir()?;
    let smelt_dir = cwd.join(".smelt");

    if !smelt_dir.exists() {
        anyhow::bail!("Smelt not initialized. Run 'smelt init' first.");
    }

    let memory_dir = smelt_dir.join("memory");
    if !memory_dir.exists() {
        std::fs::create_dir_all(&memory_dir)?;
    }

    // Use project name from directory
    let project_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let memory = SmeltMemory::open(&memory_dir)?.with_project(project_name);
    Ok(memory)
}

/// Search memory for relevant episodes
pub async fn search(query: String, limit: usize) -> Result<()> {
    let memory = get_memory()?;

    println!("Searching for: \"{}\"", query);
    println!();

    let results = memory.retrieve(&query, limit)?;

    if results.is_empty() {
        println!("No relevant episodes found.");
        println!();
        println!("Tip: Episodes are captured when you commit with 'smelt commit'.");
        return Ok(());
    }

    println!("Found {} relevant episodes:", results.len());
    println!();

    for (i, ranked) in results.iter().enumerate() {
        let ep = &ranked.episode;
        let score_pct = (ranked.score * 100.0) as u32;
        let sim_pct = (ranked.similarity * 100.0) as u32;

        println!(
            "{}. [{}] {} (score: {}%, similarity: {}%)",
            i + 1,
            &ep.id.to_string()[..8],
            ep.summary,
            score_pct,
            sim_pct
        );

        let outcome = match ep.outcome {
            EpisodeOutcome::Success => "success",
            EpisodeOutcome::Partial => "partial",
            EpisodeOutcome::Failure => "failure",
        };

        println!("   Type: {} | Outcome: {}", ep.task_type, outcome);

        if !ep.tags.is_empty() {
            println!("   Tags: {}", ep.tags.join(", "));
        }

        if !ep.files_modified.is_empty() {
            let files_preview = if ep.files_modified.len() <= 3 {
                ep.files_modified.join(", ")
            } else {
                format!(
                    "{}, ... (+{} more)",
                    ep.files_modified[..3].join(", "),
                    ep.files_modified.len() - 3
                )
            };
            println!("   Files: {}", files_preview);
        }

        if ep.feedback_count > 0 {
            println!(
                "   Feedback: {}/{} helpful",
                ep.helpful_count, ep.feedback_count
            );
        }

        println!();
    }

    Ok(())
}

/// Record feedback for an episode
pub async fn feedback(episode_id: String, helpful: bool) -> Result<()> {
    let mut memory = get_memory()?;

    // Parse the ID (allow partial match)
    let id = if episode_id.len() < 36 {
        // Partial ID - need to search for it
        // For now, try to parse as UUID prefix
        anyhow::bail!(
            "Please provide a full episode ID. Use 'smelt memory search' to find episodes."
        );
    } else {
        Uuid::parse_str(&episode_id)?
    };

    memory.record_feedback(id, helpful)?;

    if helpful {
        println!(
            "Recorded positive feedback for episode {}",
            &episode_id[..8]
        );
    } else {
        println!(
            "Recorded negative feedback for episode {}",
            &episode_id[..8]
        );
    }

    Ok(())
}

/// Show memory statistics
pub async fn stats() -> Result<()> {
    let memory = get_memory()?;
    let stats = memory.stats()?;

    println!("Memory Statistics");
    println!("-----------------");
    println!("Total episodes: {}", stats.total_episodes);
    println!("Total feedback: {}", stats.total_feedback);
    println!("Average utility: {:.2}", stats.avg_utility);

    Ok(())
}

/// Run utility propagation
pub async fn propagate(temporal: bool) -> Result<()> {
    let mut memory = get_memory()?;

    println!(
        "Running utility propagation{}...",
        if temporal {
            " with temporal credit"
        } else {
            ""
        }
    );

    let result = memory.propagate_utility(temporal)?;

    println!();
    println!("Propagation Results");
    println!("-------------------");
    println!("Episodes updated: {}", result.episodes_updated);
    println!("Total change: {:.4}", result.total_change);
    println!("Max change: {:.4}", result.max_change);

    Ok(())
}

/// Capture an episode manually (for testing)
pub async fn capture(
    summary: String,
    task_type: String,
    outcome: String,
    tags: Vec<String>,
) -> Result<()> {
    let mut memory = get_memory()?;

    let outcome = match outcome.to_lowercase().as_str() {
        "success" => EpisodeOutcome::Success,
        "partial" => EpisodeOutcome::Partial,
        "failure" => EpisodeOutcome::Failure,
        _ => anyhow::bail!("Invalid outcome. Use: success, partial, or failure"),
    };

    let episode = Episode::new(summary.clone(), task_type, outcome).with_tags(tags);

    let id = memory.capture(episode)?;

    println!("Captured episode: {} ({})", &id.to_string()[..8], summary);

    Ok(())
}

/// List all episodes
pub async fn list(limit: usize) -> Result<()> {
    let memory = get_memory()?;
    let episodes = memory.list_episodes()?;

    if episodes.is_empty() {
        println!("No episodes captured yet.");
        println!();
        println!("Tip: Episodes are captured when you commit with 'smelt commit'.");
        return Ok(());
    }

    println!("Episodes ({} total):", episodes.len());
    println!();

    for ep in episodes.iter().take(limit) {
        let outcome = match ep.outcome {
            EpisodeOutcome::Success => "✓",
            EpisodeOutcome::Partial => "~",
            EpisodeOutcome::Failure => "✗",
        };

        println!(
            "[{}] {} {} (utility: {:.2})",
            &ep.id.to_string()[..8],
            outcome,
            ep.summary,
            ep.utility
        );

        if !ep.tags.is_empty() {
            println!("     Tags: {}", ep.tags.join(", "));
        }
    }

    if episodes.len() > limit {
        println!();
        println!("... and {} more episodes", episodes.len() - limit);
    }

    Ok(())
}
