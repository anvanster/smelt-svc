// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Memory tools - Search, capture, and provide feedback on episodic memory

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use smelt_memory::types::ErrorResolution;
use smelt_memory::{Episode, EpisodeOutcome};

/// Get the tool definition for smelt_memory_search
pub fn tool_smelt_memory_search() -> Tool {
    Tool {
        name: "smelt_memory_search".to_string(),
        description: "Search Smelt's episodic memory for relevant past experiences".to_string(),
        input_schema: schema(
            json!({
                "query": {
                    "type": "string",
                    "description": "Natural language query describing what to search for"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 5)"
                }
            }),
            vec!["query"],
        ),
    }
}

/// Get the tool definition for smelt_memory_capture
pub fn tool_smelt_memory_capture() -> Tool {
    Tool {
        name: "smelt_memory_capture".to_string(),
        description: "Capture a completed task as an episode in Smelt's memory".to_string(),
        input_schema: schema(
            json!({
                "summary": {
                    "type": "string",
                    "description": "Brief summary of what was accomplished"
                },
                "task_type": {
                    "type": "string",
                    "enum": ["bugfix", "feature", "refactor", "test", "docs", "research", "debug", "setup"],
                    "description": "Type of task completed"
                },
                "outcome": {
                    "type": "string",
                    "enum": ["success", "partial", "failure"],
                    "description": "Outcome of the task"
                },
                "files_modified": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of files that were modified"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Domain tags for categorization"
                },
                "errors_resolved": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "error": { "type": "string" },
                            "resolution": { "type": "string" }
                        }
                    },
                    "description": "Errors encountered and how they were resolved"
                }
            }),
            vec!["summary", "task_type", "outcome"],
        ),
    }
}

/// Get the tool definition for smelt_memory_feedback
pub fn tool_smelt_memory_feedback() -> Tool {
    Tool {
        name: "smelt_memory_feedback".to_string(),
        description: "Record whether retrieved episodes were helpful".to_string(),
        input_schema: schema(
            json!({
                "episode_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "IDs of episodes to provide feedback on"
                },
                "helpful": {
                    "type": "boolean",
                    "description": "Whether the episodes were helpful"
                }
            }),
            vec!["episode_ids", "helpful"],
        ),
    }
}

/// Handle smelt_memory_search tool call
pub async fn handle_memory_search(
    args: &Value,
    context: &mut SmeltContext,
) -> Result<String, String> {
    // Try to load context if not already initialized
    if !context.is_initialized() {
        match context.try_load() {
            Ok(true) => {}
            Ok(false) => {
                return Err("Smelt is not initialized. Run smelt_init first.".to_string());
            }
            Err(e) => {
                return Err(format!("Failed to load Smelt: {}", e));
            }
        }
    }

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: query")?;

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(5);

    let memory = context.memory().map_err(|e| e.to_string())?;
    let results = memory.retrieve(query, limit).map_err(|e| e.to_string())?;

    if results.is_empty() {
        return Ok("No relevant episodes found in memory.".to_string());
    }

    let mut output = format!("Found {} relevant past experience(s):\n\n", results.len());

    for (i, ranked) in results.iter().enumerate() {
        let ep = &ranked.episode;

        output.push_str(&format!("{}. **{}**\n", i + 1, ep.summary));
        output.push_str(&format!("   - ID: {}\n", &ep.id.to_string()[..8]));
        output.push_str(&format!("   - Type: {}\n", ep.task_type));
        output.push_str(&format!("   - Outcome: {:?}\n", ep.outcome));
        output.push_str(&format!(
            "   - Relevance: {:.0}% similarity, {:.0}% utility\n",
            ranked.similarity * 100.0,
            ranked.score * 100.0
        ));

        if !ep.files_modified.is_empty() {
            let files: Vec<&str> = ep
                .files_modified
                .iter()
                .map(|f| f.split('/').next_back().unwrap_or(f))
                .take(3)
                .collect();
            output.push_str(&format!("   - Files: {}\n", files.join(", ")));
        }

        if !ep.tags.is_empty() {
            output.push_str(&format!("   - Tags: {}\n", ep.tags.join(", ")));
        }

        // Show resolved errors
        if !ep.errors_resolved.is_empty() {
            output.push_str("   - Errors resolved:\n");
            for err in ep.errors_resolved.iter().take(2) {
                output.push_str(&format!("     - {}\n", truncate(&err.error, 50)));
                output.push_str(&format!(
                    "       Resolution: {}\n",
                    truncate(&err.resolution, 50)
                ));
            }
        }

        output.push('\n');
    }

    output.push_str("Use smelt_memory_feedback to indicate if these were helpful.");

    Ok(output)
}

/// Handle smelt_memory_capture tool call
pub async fn handle_memory_capture(
    args: &Value,
    context: &mut SmeltContext,
) -> Result<String, String> {
    // Try to load context if not already initialized
    if !context.is_initialized() {
        match context.try_load() {
            Ok(true) => {}
            Ok(false) => {
                return Err("Smelt is not initialized. Run smelt_init first.".to_string());
            }
            Err(e) => {
                return Err(format!("Failed to load Smelt: {}", e));
            }
        }
    }

    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: summary")?;

    let task_type = args
        .get("task_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_type")?;

    let outcome_str = args
        .get("outcome")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: outcome")?;

    let files_modified: Vec<String> = args
        .get("files_modified")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let tags: Vec<String> = args
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let errors_resolved: Vec<ErrorResolution> = args
        .get("errors_resolved")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let error = v.get("error")?.as_str()?.to_string();
                    let resolution = v.get("resolution")?.as_str()?.to_string();
                    Some(ErrorResolution { error, resolution })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse outcome
    let outcome = match outcome_str {
        "success" => EpisodeOutcome::Success,
        "partial" => EpisodeOutcome::Partial,
        "failure" => EpisodeOutcome::Failure,
        _ => EpisodeOutcome::Partial,
    };

    // Create episode
    let mut episode = Episode::new(summary.to_string(), task_type.to_string(), outcome)
        .with_files(files_modified)
        .with_tags(tags)
        .with_errors(errors_resolved);

    // Set project if available - get it before borrowing memory mutably
    let project = context.project().map(String::from);
    if let Some(ref proj) = project {
        episode = episode.with_project(proj.clone());
    }

    // Capture the episode
    let memory = context.memory_mut().map_err(|e| e.to_string())?;
    let id = memory.capture(episode).map_err(|e| e.to_string())?;

    let mut output = String::from("✅ Episode captured successfully!\n\n");
    output.push_str(&format!("ID: {}\n", &id.to_string()[..8]));
    output.push_str(&format!("Summary: {}\n", summary));
    output.push_str(&format!("Type: {}\n", task_type));
    output.push_str(&format!("Outcome: {:?}\n", outcome));

    if let Some(ref proj) = project {
        output.push_str(&format!("Project: {}\n", proj));
    }

    output.push_str("\nThis experience is now stored for future reference.");

    // Auto-propagate utility
    output.push_str("\n\n📈 Running utility propagation...\n");
    match memory.propagate_utility(false) {
        Ok(result) => {
            output.push_str(&format!(
                "  Updated {} episodes, total change: {:.3}\n",
                result.episodes_updated, result.total_change
            ));
        }
        Err(e) => {
            output.push_str(&format!("  (propagation skipped: {})\n", e));
        }
    }

    Ok(output)
}

/// Handle smelt_memory_feedback tool call
pub async fn handle_memory_feedback(
    args: &Value,
    context: &mut SmeltContext,
) -> Result<String, String> {
    // Try to load context if not already initialized
    if !context.is_initialized() {
        match context.try_load() {
            Ok(true) => {}
            Ok(false) => {
                return Err("Smelt is not initialized. Run smelt_init first.".to_string());
            }
            Err(e) => {
                return Err(format!("Failed to load Smelt: {}", e));
            }
        }
    }

    let episode_ids: Vec<String> = args
        .get("episode_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .ok_or("Missing required parameter: episode_ids")?;

    let helpful = args
        .get("helpful")
        .and_then(|v| v.as_bool())
        .ok_or("Missing required parameter: helpful")?;

    if episode_ids.is_empty() {
        return Err("episode_ids array is empty".to_string());
    }

    let memory = context.memory_mut().map_err(|e| e.to_string())?;
    let mut updated = 0;

    for id_str in &episode_ids {
        // Try to parse as UUID (full or partial)
        let episodes = memory.list_episodes().map_err(|e| e.to_string())?;

        for ep in episodes {
            let ep_id_str = ep.id.to_string();
            if ep_id_str.starts_with(id_str) || ep_id_str[..8] == *id_str {
                memory
                    .record_feedback(ep.id, helpful)
                    .map_err(|e| e.to_string())?;
                updated += 1;
                break;
            }
        }
    }

    let feedback_type = if helpful { "helpful" } else { "not helpful" };

    let mut output = format!(
        "Feedback recorded: {} episode(s) marked as {}.\n",
        updated, feedback_type
    );
    output.push_str("This helps improve future retrieval quality.");

    // Run propagation after feedback
    output.push_str("\n\n📈 Running utility propagation...\n");
    match memory.propagate_utility(false) {
        Ok(result) => {
            output.push_str(&format!(
                "  Updated {} episodes, total change: {:.3}\n",
                result.episodes_updated, result.total_change
            ));
        }
        Err(e) => {
            output.push_str(&format!("  (propagation skipped: {})\n", e));
        }
    }

    Ok(output)
}

/// Truncate a string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_memory_capture() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize directly in the temp directory
        context.initialize(Some(dir.path())).unwrap();

        // Capture episode
        let result = handle_memory_capture(
            &json!({
                "summary": "Fixed login timeout bug",
                "task_type": "bugfix",
                "outcome": "success",
                "files_modified": ["src/auth/login.rs"],
                "tags": ["auth", "security"],
                "errors_resolved": [{
                    "error": "Connection timeout after 10s",
                    "resolution": "Increased timeout to 30s"
                }]
            }),
            &mut context,
        )
        .await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let output = result.unwrap();
        assert!(output.contains("Episode captured successfully"));
    }

    #[tokio::test]
    async fn test_memory_search_empty() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize directly in the temp directory
        context.initialize(Some(dir.path())).unwrap();

        // Search (should be empty)
        let result = handle_memory_search(
            &json!({
                "query": "authentication login"
            }),
            &mut context,
        )
        .await;

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        assert!(result.unwrap().contains("No relevant episodes"));
    }
}
