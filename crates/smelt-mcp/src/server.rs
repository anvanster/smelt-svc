// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! MCP Server implementation

use crate::context::SmeltContext;
use crate::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
    ServerInfo, ToolsCapability,
};
use crate::tools;
use serde_json::{json, Value};

/// MCP Server implementation
pub struct McpServer {
    /// Whether the server has been initialized
    initialized: bool,

    /// Shared context for tool operations
    context: SmeltContext,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(context: SmeltContext) -> Self {
        Self {
            initialized: false,
            context,
        }
    }

    /// Handle an incoming JSON-RPC request
    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone().unwrap_or(Value::Null);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.params),
            "initialized" => self.handle_initialized(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&request.params).await,
            "shutdown" => self.handle_shutdown(),
            _ => Err(JsonRpcError::method_not_found(&request.method)),
        };

        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(error) => JsonRpcResponse::error(id, error),
        }
    }

    /// Handle initialize request
    fn handle_initialize(&mut self, _params: &Value) -> Result<Value, JsonRpcError> {
        self.initialized = true;

        // Try to load existing smelt context
        let _ = self.context.try_load();

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {},
            },
            server_info: ServerInfo {
                name: "smelt-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    /// Handle initialized notification
    fn handle_initialized(&self) -> Result<Value, JsonRpcError> {
        Ok(json!({}))
    }

    /// Handle tools/list request
    fn handle_tools_list(&self) -> Result<Value, JsonRpcError> {
        let tool_list = tools::get_tools();
        Ok(json!({ "tools": tool_list }))
    }

    /// Handle tools/call request
    async fn handle_tools_call(&mut self, params: &Value) -> Result<Value, JsonRpcError> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing tool name"))?;

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = tools::execute_tool(name, &arguments, &mut self.context).await;

        Ok(serde_json::to_value(result).unwrap())
    }

    /// Handle shutdown request
    fn handle_shutdown(&mut self) -> Result<Value, JsonRpcError> {
        self.initialized = false;
        Ok(json!({}))
    }

    /// Check if the server is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_request(method: &str, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: method.to_string(),
            params,
        }
    }

    #[tokio::test]
    async fn test_initialize() {
        let context = SmeltContext::new();
        let mut server = McpServer::new(context);

        assert!(!server.is_initialized());

        let request = make_request("initialize", json!({}));
        let response = server.handle_request(request).await;

        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert!(server.is_initialized());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "smelt-mcp");
    }

    #[tokio::test]
    async fn test_tools_list() {
        let context = SmeltContext::new();
        let mut server = McpServer::new(context);

        let request = make_request("tools/list", json!({}));
        let response = server.handle_request(request).await;

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();

        // Should have 10 tools
        assert_eq!(tools.len(), 10);

        // Check tool names
        let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

        assert!(tool_names.contains(&"smelt_init"));
        assert!(tool_names.contains(&"smelt_status"));
        assert!(tool_names.contains(&"smelt_validate"));
        assert!(tool_names.contains(&"smelt_commit"));
        assert!(tool_names.contains(&"smelt_intent_create"));
        assert!(tool_names.contains(&"smelt_intent_list"));
        assert!(tool_names.contains(&"smelt_intent_abandon"));
        assert!(tool_names.contains(&"smelt_memory_search"));
        assert!(tool_names.contains(&"smelt_memory_capture"));
        assert!(tool_names.contains(&"smelt_memory_feedback"));
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let context = SmeltContext::new();
        let mut server = McpServer::new(context);

        let request = make_request("unknown/method", json!({}));
        let response = server.handle_request(request).await;

        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, JsonRpcError::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let context = SmeltContext::new();
        let mut server = McpServer::new(context);

        // Initialize
        let init_request = make_request("initialize", json!({}));
        let _ = server.handle_request(init_request).await;
        assert!(server.is_initialized());

        // Shutdown
        let shutdown_request = make_request("shutdown", json!({}));
        let response = server.handle_request(shutdown_request).await;

        assert!(response.result.is_some());
        assert!(!server.is_initialized());
    }
}
