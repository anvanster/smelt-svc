//! MCP Tool implementations
//!
//! This module contains all the tool handlers for the Smelt MCP server.

mod commit;
mod init;
mod intent;
mod memory;
mod status;
mod validate;

use crate::context::SmeltContext;
use crate::protocol::{Tool, ToolResult};
use serde_json::{json, Value};

pub use commit::*;
pub use init::*;
pub use intent::*;
pub use memory::*;
pub use status::*;
pub use validate::*;

/// Get all tool definitions
pub fn get_tools() -> Vec<Tool> {
    vec![
        // Core tools
        tool_smelt_init(),
        tool_smelt_status(),
        tool_smelt_validate(),
        tool_smelt_commit(),
        // Intent tools
        tool_smelt_intent_create(),
        tool_smelt_intent_list(),
        tool_smelt_intent_abandon(),
        // Memory tools
        tool_smelt_memory_search(),
        tool_smelt_memory_capture(),
        tool_smelt_memory_feedback(),
    ]
}

/// Execute a tool by name
pub async fn execute_tool(name: &str, args: &Value, context: &mut SmeltContext) -> ToolResult {
    let result = match name {
        // Core tools
        "smelt_init" => handle_init(args, context).await,
        "smelt_status" => handle_status(args, context).await,
        "smelt_validate" => handle_validate(args, context).await,
        "smelt_commit" => handle_commit(args, context).await,
        // Intent tools
        "smelt_intent_create" => handle_intent_create(args, context).await,
        "smelt_intent_list" => handle_intent_list(args, context).await,
        "smelt_intent_abandon" => handle_intent_abandon(args, context).await,
        // Memory tools
        "smelt_memory_search" => handle_memory_search(args, context).await,
        "smelt_memory_capture" => handle_memory_capture(args, context).await,
        "smelt_memory_feedback" => handle_memory_feedback(args, context).await,
        // Unknown tool
        _ => Err(format!("Unknown tool: {}", name)),
    };

    match result {
        Ok(text) => ToolResult::success(text),
        Err(e) => ToolResult::error(format!("Error: {}", e)),
    }
}

/// Helper to build tool schema
fn schema(properties: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}
