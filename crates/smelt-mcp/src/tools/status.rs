//! smelt_status tool - Get semantic status of working directory

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use smelt_core::IntentStatus;

/// Get the tool definition for smelt_status
pub fn tool_smelt_status() -> Tool {
    Tool {
        name: "smelt_status".to_string(),
        description:
            "Show semantic status of working directory including active intents and changes"
                .to_string(),
        input_schema: schema(
            json!({
                "intent_id": {
                    "type": "string",
                    "description": "Intent ID to show status for (optional)"
                },
                "detailed": {
                    "type": "boolean",
                    "description": "Include detailed change information"
                }
            }),
            vec![],
        ),
    }
}

/// Handle smelt_status tool call
pub async fn handle_status(args: &Value, context: &mut SmeltContext) -> Result<String, String> {
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

    let intent_id = args.get("intent_id").and_then(|v| v.as_str());
    let detailed = args
        .get("detailed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let storage = context.storage().map_err(|e| e.to_string())?;

    let mut output = String::from("📊 Smelt Status\n");
    output.push_str("===============\n\n");

    // Show project info
    if let Some(project) = context.project() {
        output.push_str(&format!("Project: {}\n", project));
    }

    // If specific intent requested
    if let Some(id_prefix) = intent_id {
        let intent = storage
            .find_intent_by_prefix(id_prefix)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Intent not found: {}", id_prefix))?;

        output.push_str(&format!("\n📋 Intent: {}\n", &intent.id.to_string()[..8]));
        output.push_str(&format!("  Goal: {}\n", intent.goal));
        output.push_str(&format!("  Status: {:?}\n", intent.status));
        output.push_str(&format!(
            "  Created: {}\n",
            intent.created_at.format("%Y-%m-%d %H:%M")
        ));

        if !intent.constraints.is_empty() {
            output.push_str("  Constraints:\n");
            for c in &intent.constraints {
                output.push_str(&format!("    - {}: {}\n", c.name, c.value));
            }
        }

        if detailed {
            // Show deltas for this intent
            let deltas = storage
                .get_deltas_for_intent(intent.id)
                .map_err(|e| e.to_string())?;

            if !deltas.is_empty() {
                output.push_str(&format!("\n  Deltas: {} recorded\n", deltas.len()));
                for delta in &deltas {
                    output.push_str(&format!("    - {} changes\n", delta.changes.len()));
                }
            }
        }

        return Ok(output);
    }

    // Show active intents
    let intents = storage.list_intents(None).map_err(|e| e.to_string())?;

    let active: Vec<_> = intents.iter().filter(|i| i.status.is_active()).collect();
    let completed: Vec<_> = intents
        .iter()
        .filter(|i| matches!(i.status, IntentStatus::Committed { .. }))
        .collect();

    output.push_str(&format!("\n📋 Active Intents: {}\n", active.len()));

    if active.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for intent in &active {
            let status_icon = match &intent.status {
                IntentStatus::Draft => "📝",
                IntentStatus::InProgress => "🔄",
                IntentStatus::PendingValidation => "⏳",
                IntentStatus::Validated => "✅",
                _ => "📋",
            };

            output.push_str(&format!(
                "  {} {} - {}\n",
                status_icon,
                &intent.id.to_string()[..8],
                truncate(&intent.goal, 50)
            ));
        }
    }

    output.push_str(&format!("\n✅ Completed: {}\n", completed.len()));

    // Show graph stats
    if let Ok(graph) = context.graph() {
        output.push_str(&format!(
            "\n📈 Graph: {} nodes, {} edges\n",
            graph.node_count(),
            graph.edge_count()
        ));
    }

    // Show memory stats
    if let Ok(memory) = context.memory() {
        if let Ok(stats) = memory.stats() {
            output.push_str(&format!("🧠 Memory: {} episodes\n", stats.total_episodes));
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
    async fn test_status_not_initialized() {
        let _dir = tempdir().unwrap();
        let context = SmeltContext::new();

        // Context is not initialized
        assert!(!context.is_initialized());
    }

    #[tokio::test]
    async fn test_status_initialized() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize
        context.initialize(Some(dir.path())).unwrap();

        // Should work now
        let result = handle_status(&json!({}), &mut context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Smelt Status"));
    }
}
