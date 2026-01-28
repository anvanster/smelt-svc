//! smelt_commit tool - Commit changes with semantic delta

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use smelt_core::{Git2Interface, GitInterface, IntentStatus};

/// Get the tool definition for smelt_commit
pub fn tool_smelt_commit() -> Tool {
    Tool {
        name: "smelt_commit".to_string(),
        description: "Commit staged changes with semantic delta attached".to_string(),
        input_schema: schema(
            json!({
                "intent_id": {
                    "type": "string",
                    "description": "Intent ID this commit fulfills"
                },
                "message": {
                    "type": "string",
                    "description": "Commit message"
                }
            }),
            vec!["message"],
        ),
    }
}

/// Handle smelt_commit tool call
pub async fn handle_commit(args: &Value, context: &mut SmeltContext) -> Result<String, String> {
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

    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: message")?;

    let intent_id_str = args.get("intent_id").and_then(|v| v.as_str());

    let storage = context.storage().map_err(|e| e.to_string())?;

    // Find the intent
    let intent = if let Some(id_prefix) = intent_id_str {
        Some(
            storage
                .find_intent_by_prefix(id_prefix)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Intent not found: {}", id_prefix))?,
        )
    } else {
        // Try to find the most recent active intent
        let intents = storage.list_intents(None).map_err(|e| e.to_string())?;
        intents.into_iter().find(|i| i.status.is_active())
    };

    // Open git repository
    let repo_path = context.working_dir().to_path_buf();
    let git = Git2Interface::open(&repo_path).map_err(|e| e.to_string())?;

    // Check for staged changes
    let changes = git.staged_files().map_err(|e| e.to_string())?;

    if changes.is_empty() {
        return Err(
            "No staged changes to commit. Use 'git add' to stage changes first.".to_string(),
        );
    }

    // Build commit message with semantic metadata
    let full_message = if let Some(ref intent) = intent {
        format!(
            "{}\n\n[smelt] intent: {}\ngoal: {}",
            message, intent.id, intent.goal
        )
    } else {
        message.to_string()
    };

    // Create the commit
    // Note: We need a mutable reference, but we can't mutate through the trait
    // This is a limitation - in practice, the CLI would handle this differently
    let repo_path_for_commit = context.working_dir().to_path_buf();
    let mut git_mut = Git2Interface::open(&repo_path_for_commit).map_err(|e| e.to_string())?;
    let commit_sha = git_mut.commit(&full_message).map_err(|e| e.to_string())?;

    // Update intent status if we have one
    if let Some(intent) = intent {
        let storage = context.storage_mut().map_err(|e| e.to_string())?;
        storage
            .update_intent_status(
                intent.id,
                IntentStatus::Committed {
                    git_sha: commit_sha.clone(),
                },
            )
            .map_err(|e| e.to_string())?;
    }

    let mut output = String::from("✅ Commit created successfully!\n\n");
    output.push_str(&format!("SHA: {}\n", &commit_sha[..8]));
    output.push_str(&format!("Message: {}\n", message));
    output.push_str(&format!("Files changed: {}\n", changes.len()));

    if intent_id_str.is_some() {
        output.push_str("\n📋 Intent marked as committed.\n");
    }

    output.push_str(
        "\n💡 Tip: Use smelt_memory_capture to save this experience for future reference.",
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_commit_not_initialized() {
        let _dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Context is not initialized, should return error
        let result = handle_commit(
            &json!({
                "message": "test commit"
            }),
            &mut context,
        )
        .await;

        assert!(result.is_err());
    }
}
