//! MCP Protocol types - JSON-RPC 2.0 implementation
//!
//! This module defines the core types for the Model Context Protocol (MCP)
//! JSON-RPC communication layer.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    #[allow(dead_code)]
    pub jsonrpc: String,

    /// Request ID (null for notifications)
    pub id: Option<Value>,

    /// Method name
    pub method: String,

    /// Method parameters
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request ID (matches request)
    pub id: Value,

    /// Result on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Standard error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Create a parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: format!("Parse error: {}", message.into()),
            data: None,
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: format!("Invalid request: {}", message.into()),
            data: None,
        }
    }

    /// Create a method not found error
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Create an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: format!("Invalid params: {}", message.into()),
            data: None,
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: format!("Internal error: {}", message.into()),
            data: None,
        }
    }
}

/// MCP Tool definition
#[derive(Debug, Serialize)]
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP Server capabilities
#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    /// Tools capability
    pub tools: ToolsCapability,
}

/// Tools capability marker
#[derive(Debug, Serialize)]
pub struct ToolsCapability {}

/// MCP Server info
#[derive(Debug, Serialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,
}

/// Initialize result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server info
    pub server_info: ServerInfo,
}

/// Tool call result content
#[derive(Debug, Serialize)]
pub struct ToolContent {
    /// Content type (always "text" for now)
    #[serde(rename = "type")]
    pub content_type: String,

    /// Text content
    pub text: String,
}

/// Tool call result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Content array
    pub content: Vec<ToolContent>,

    /// Whether this is an error result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a success result
    pub fn success(text: String) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: None,
        }
    }

    /// Create an error result
    pub fn error(text: String) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.method, "tools/list");
        assert_eq!(request.id, Some(json!(1)));
    }

    #[test]
    fn test_serialize_response() {
        let response = JsonRpcResponse::success(json!(1), json!({"result": "ok"}));
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_serialize_error() {
        let response = JsonRpcResponse::error(json!(1), JsonRpcError::method_not_found("unknown"));
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"error\""));
        assert!(json.contains("-32601"));
    }

    #[test]
    fn test_tool_result() {
        let result = ToolResult::success("Hello, world!".to_string());
        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello, world!"));
        assert!(!json.contains("isError"));

        let error_result = ToolResult::error("Something went wrong".to_string());
        let error_json = serde_json::to_string(&error_result).unwrap();

        assert!(error_json.contains("\"isError\":true"));
    }
}
