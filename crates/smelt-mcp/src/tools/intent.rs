//! Intent tools - Create and list intents

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use smelt_core::{Author, AuthorType, Constraint, ContextLinks, IntentRecord, IntentStatus};
use uuid::Uuid;

/// Get the tool definition for smelt_intent_create
pub fn tool_smelt_intent_create() -> Tool {
    Tool {
        name: "smelt_intent_create".to_string(),
        description: "Create a new intent describing planned changes".to_string(),
        input_schema: schema(
            json!({
                "goal": {
                    "type": "string",
                    "description": "The goal of this intent"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of planned changes"
                },
                "constraints": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Architectural constraints to enforce"
                }
            }),
            vec!["goal"],
        ),
    }
}

/// Get the tool definition for smelt_intent_list
pub fn tool_smelt_intent_list() -> Tool {
    Tool {
        name: "smelt_intent_list".to_string(),
        description: "List all intents with optional status filter".to_string(),
        input_schema: schema(
            json!({
                "status": {
                    "type": "string",
                    "enum": ["active", "completed", "abandoned", "all"],
                    "description": "Filter by status (defaults to 'all')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of intents to return"
                }
            }),
            vec![],
        ),
    }
}

/// Get the tool definition for smelt_intent_abandon
pub fn tool_smelt_intent_abandon() -> Tool {
    Tool {
        name: "smelt_intent_abandon".to_string(),
        description: "Abandon an intent that is no longer needed".to_string(),
        input_schema: schema(
            json!({
                "intent_id": {
                    "type": "string",
                    "description": "Intent ID (or prefix) to abandon"
                }
            }),
            vec!["intent_id"],
        ),
    }
}

/// Handle smelt_intent_create tool call
pub async fn handle_intent_create(
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

    let goal = args
        .get("goal")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: goal")?;

    let description = args.get("description").and_then(|v| v.as_str());

    let constraints: Vec<String> = args
        .get("constraints")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Create baseline snapshot
    let graph = context.graph_mut().map_err(|e| e.to_string())?;
    let intent_id = Uuid::new_v4();
    let baseline_snapshot_id = graph
        .snapshot_for_intent(intent_id)
        .map_err(|e| e.to_string())?;

    // Create the intent record
    let intent = IntentRecord {
        id: intent_id,
        created_at: chrono::Utc::now(),
        author: Author {
            name: "AI Assistant".to_string(),
            email: "ai@smelt.dev".to_string(),
            author_type: AuthorType::AI,
        },
        goal: goal.to_string(),
        rationale: description.map(String::from),
        constraints: constraints
            .into_iter()
            .map(|c| Constraint {
                name: "user".to_string(),
                value: c,
                required: true,
            })
            .collect(),
        context_links: ContextLinks::default(),
        status: IntentStatus::InProgress,
        baseline_snapshot_id: Some(baseline_snapshot_id),
    };

    // Store the intent
    let storage = context.storage().map_err(|e| e.to_string())?;
    storage.store_intent(&intent).map_err(|e| e.to_string())?;

    let mut output = String::from("✅ Intent created successfully!\n\n");
    output.push_str(&format!("ID: {}\n", &intent.id.to_string()[..8]));
    output.push_str(&format!("Goal: {}\n", intent.goal));
    output.push_str(&format!("Status: {:?}\n", intent.status));

    if let Some(desc) = description {
        output.push_str(&format!("Description: {}\n", desc));
    }

    if !intent.constraints.is_empty() {
        output.push_str("\nConstraints:\n");
        for c in &intent.constraints {
            output.push_str(&format!("  - {}\n", c.value));
        }
    }

    output.push_str("\nNext steps:\n");
    output.push_str("  1. Make your changes\n");
    output.push_str("  2. Check status: smelt_status\n");
    output.push_str("  3. Validate: smelt_validate\n");
    output.push_str("  4. Commit: smelt_commit\n");

    Ok(output)
}

/// Handle smelt_intent_list tool call
pub async fn handle_intent_list(
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

    let status_filter = args.get("status").and_then(|v| v.as_str()).unwrap_or("all");

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(50);

    let storage = context.storage().map_err(|e| e.to_string())?;
    let intents = storage.list_intents(None).map_err(|e| e.to_string())?;

    // Filter by status
    let filtered: Vec<_> = intents
        .into_iter()
        .filter(|i| match status_filter {
            "active" => i.status.is_active(),
            "completed" => matches!(i.status, IntentStatus::Committed { .. }),
            "abandoned" => matches!(i.status, IntentStatus::Abandoned),
            _ => true,
        })
        .take(limit)
        .collect();

    let mut output = String::from("📋 Intents\n");
    output.push_str("==========\n\n");

    if filtered.is_empty() {
        output.push_str(&format!(
            "No intents found with status: {}\n",
            status_filter
        ));
        output.push_str("\nCreate one with smelt_intent_create.");
        return Ok(output);
    }

    output.push_str(&format!(
        "Showing {} intent(s) (filter: {})\n\n",
        filtered.len(),
        status_filter
    ));

    for intent in &filtered {
        let status_icon = match &intent.status {
            IntentStatus::Draft => "📝",
            IntentStatus::InProgress => "🔄",
            IntentStatus::PendingValidation => "⏳",
            IntentStatus::Validated => "✅",
            IntentStatus::Committed { .. } => "✔️",
            IntentStatus::Rejected { .. } => "❌",
            IntentStatus::Abandoned => "🗑️",
        };

        output.push_str(&format!(
            "{} **{}**\n",
            status_icon,
            &intent.id.to_string()[..8]
        ));
        output.push_str(&format!("   Goal: {}\n", truncate(&intent.goal, 60)));
        output.push_str(&format!("   Status: {:?}\n", intent.status));
        output.push_str(&format!(
            "   Created: {}\n",
            intent.created_at.format("%Y-%m-%d %H:%M")
        ));

        if !intent.constraints.is_empty() {
            output.push_str(&format!("   Constraints: {}\n", intent.constraints.len()));
        }

        output.push('\n');
    }

    Ok(output)
}

/// Handle smelt_intent_abandon tool call
pub async fn handle_intent_abandon(
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

    let intent_id = args
        .get("intent_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: intent_id")?;

    let storage = context.storage().map_err(|e| e.to_string())?;

    // Find intent by prefix
    let intent = storage
        .find_intent_by_prefix(intent_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Intent not found: {}", intent_id))?;

    // Check if intent can be abandoned
    match &intent.status {
        IntentStatus::Committed { .. } => {
            return Err("Cannot abandon a committed intent.".to_string());
        }
        IntentStatus::Abandoned => {
            return Err("Intent is already abandoned.".to_string());
        }
        _ => {}
    }

    // Update status to abandoned
    storage
        .update_intent_status(intent.id, IntentStatus::Abandoned)
        .map_err(|e| e.to_string())?;

    let mut output = String::from("🗑️ Intent abandoned successfully!\n\n");
    output.push_str(&format!("ID: {}\n", &intent.id.to_string()[..8]));
    output.push_str(&format!("Goal: {}\n", intent.goal));
    output.push_str("Status: Abandoned\n");

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
    async fn test_intent_create() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize directly
        context.initialize(Some(dir.path())).unwrap();

        // Create intent
        let result = handle_intent_create(
            &json!({
                "goal": "Add user authentication",
                "description": "Implement OAuth2 login flow",
                "constraints": ["no_breaking_changes", "maintain_api_compatibility"]
            }),
            &mut context,
        )
        .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Intent created successfully"));
        assert!(output.contains("Add user authentication"));
    }

    #[tokio::test]
    async fn test_intent_list_empty() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize directly
        context.initialize(Some(dir.path())).unwrap();

        // List (should be empty)
        let result = handle_intent_list(&json!({}), &mut context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No intents found"));
    }
}
