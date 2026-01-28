//! smelt_init tool - Initialize Smelt in a repository

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use std::path::Path;

/// Get the tool definition for smelt_init
pub fn tool_smelt_init() -> Tool {
    Tool {
        name: "smelt_init".to_string(),
        description: "Initialize Smelt semantic version control in a Git repository".to_string(),
        input_schema: schema(
            json!({
                "path": {
                    "type": "string",
                    "description": "Repository path (defaults to current directory)"
                }
            }),
            vec![],
        ),
    }
}

/// Handle smelt_init tool call
pub async fn handle_init(args: &Value, context: &mut SmeltContext) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str()).map(Path::new);

    // Check if already initialized
    if context.is_initialized() {
        return Ok("Smelt is already initialized in this directory.".to_string());
    }

    // Try to load existing
    match context.try_load() {
        Ok(true) => {
            return Ok(
                "Smelt initialized successfully (loaded existing configuration).".to_string(),
            );
        }
        Ok(false) => {
            // Not initialized, continue to initialize
        }
        Err(e) => {
            return Err(format!("Failed to check existing configuration: {}", e));
        }
    }

    // Initialize new
    context.initialize(path).map_err(|e| e.to_string())?;

    let mut output = String::from("✅ Smelt initialized successfully!\n\n");
    output.push_str("Created .smelt/ directory with:\n");
    output.push_str("  - smelt.db (intent and delta storage)\n");
    output.push_str("  - graph/ (code graph)\n");
    output.push_str("  - memory/ (episodic memory)\n");
    output.push_str("\nNext steps:\n");
    output.push_str("  1. Create an intent: smelt_intent_create\n");
    output.push_str("  2. Make your changes\n");
    output.push_str("  3. Check status: smelt_status\n");
    output.push_str("  4. Validate changes: smelt_validate\n");
    output.push_str("  5. Commit: smelt_commit\n");

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_init() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize using context.initialize directly (simulates the path being set)
        context.initialize(Some(dir.path())).unwrap();

        assert!(context.is_initialized());
        assert!(context.smelt_dir().unwrap().exists());
    }

    #[tokio::test]
    async fn test_init_already_initialized() {
        let dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Initialize directly with path
        context.initialize(Some(dir.path())).unwrap();

        // Second init should report already initialized
        let result = handle_init(&json!({}), &mut context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("already initialized"));
    }
}
