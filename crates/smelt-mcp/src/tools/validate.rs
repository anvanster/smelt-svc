//! smelt_validate tool - Validate changes against architectural constraints

use super::schema;
use crate::context::SmeltContext;
use crate::protocol::Tool;
use serde_json::{json, Value};
use smelt_core::{ImpactSummary, SemanticDelta};
use smelt_validator::ValidationSeverity;
use uuid::Uuid;

/// Get the tool definition for smelt_validate
pub fn tool_smelt_validate() -> Tool {
    Tool {
        name: "smelt_validate".to_string(),
        description: "Validate semantic changes against intent constraints and architecture rules"
            .to_string(),
        input_schema: schema(
            json!({
                "intent_id": {
                    "type": "string",
                    "description": "Intent ID to validate against"
                }
            }),
            vec![],
        ),
    }
}

/// Handle smelt_validate tool call
pub async fn handle_validate(args: &Value, context: &mut SmeltContext) -> Result<String, String> {
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

    let intent_id_str = args.get("intent_id").and_then(|v| v.as_str());

    let validator = context.validator().map_err(|e| e.to_string())?;
    let storage = context.storage().map_err(|e| e.to_string())?;

    // Get the intent if specified
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

    // Create a delta to validate
    // In a real implementation, this would compute the actual semantic delta
    // from the current working directory changes
    let delta = if let Some(ref intent) = intent {
        // Get latest delta for this intent, or create empty one
        let deltas = storage
            .get_deltas_for_intent(intent.id)
            .map_err(|e| e.to_string())?;

        deltas.into_iter().next().unwrap_or_else(|| SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            timestamp: chrono::Utc::now(),
            from_snapshot: Uuid::nil(),
            to_snapshot: Uuid::nil(),
            changes: vec![],
            impact_summary: ImpactSummary::default(),
        })
    } else {
        // Create empty delta for validation
        SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::nil(),
            timestamp: chrono::Utc::now(),
            from_snapshot: Uuid::nil(),
            to_snapshot: Uuid::nil(),
            changes: vec![],
            impact_summary: ImpactSummary::default(),
        }
    };

    // Run validation
    let outcome = validator.validate(&delta, intent.as_ref());

    let mut output = String::from("🔍 Validation Results\n");
    output.push_str("=====================\n\n");

    if let Some(ref intent) = intent {
        output.push_str(&format!(
            "Intent: {} ({})\n\n",
            &intent.id.to_string()[..8],
            truncate(&intent.goal, 40)
        ));
    }

    if outcome.passed {
        output.push_str("✅ Validation PASSED\n\n");
    } else {
        output.push_str("❌ Validation FAILED\n\n");
    }

    output.push_str(&format!(
        "Summary: {} errors, {} warnings, {} info\n\n",
        outcome.error_count, outcome.warning_count, outcome.info_count
    ));

    // Show violations grouped by severity
    if outcome.error_count > 0 {
        output.push_str("🚫 Errors:\n");
        for v in outcome.errors() {
            output.push_str(&format!("  - [{}] {}\n", v.rule, v.message));
            if let Some(ref loc) = v.location {
                output.push_str(&format!("    at: {}\n", loc));
            }
            if let Some(ref suggestion) = v.suggestion {
                output.push_str(&format!("    💡 {}\n", suggestion));
            }
        }
        output.push('\n');
    }

    if outcome.warning_count > 0 {
        output.push_str("⚠️ Warnings:\n");
        for v in outcome.warnings() {
            output.push_str(&format!("  - [{}] {}\n", v.rule, v.message));
            if let Some(ref suggestion) = v.suggestion {
                output.push_str(&format!("    💡 {}\n", suggestion));
            }
        }
        output.push('\n');
    }

    // Show info messages only if detailed
    let info_violations: Vec<_> = outcome
        .violations
        .iter()
        .filter(|v| v.severity == ValidationSeverity::Info)
        .collect();

    if !info_violations.is_empty() {
        output.push_str("ℹ️ Info:\n");
        for v in info_violations.iter().take(5) {
            output.push_str(&format!("  - {}\n", v.message));
        }
        if info_violations.len() > 5 {
            output.push_str(&format!("  ... and {} more\n", info_violations.len() - 5));
        }
    }

    if outcome.passed {
        output.push_str("\n✅ Ready to commit with smelt_commit\n");
    } else {
        output.push_str("\n❌ Please fix the errors before committing\n");
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
    async fn test_validate_not_initialized() {
        let _dir = tempdir().unwrap();
        let mut context = SmeltContext::new();

        // Context is not initialized, should return error
        let result = handle_validate(&json!({}), &mut context).await;
        assert!(result.is_err());
    }
}
