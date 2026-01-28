//! Error types for the MCP server

use thiserror::Error;

/// MCP Server errors
#[derive(Debug, Error)]
pub enum McpError {
    /// Smelt core error
    #[error("Smelt error: {0}")]
    Smelt(#[from] smelt_core::SmeltError),

    /// Memory system error
    #[error("Memory error: {0}")]
    Memory(#[from] smelt_memory::MemoryError),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(#[from] smelt_validator::ValidationError),

    /// JSON serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Not initialized
    #[error("Smelt not initialized in this directory")]
    NotInitialized,

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),

    /// Tool not found
    #[error("Unknown tool: {0}")]
    ToolNotFound(String),

    /// Git error
    #[error("Git error: {0}")]
    Git(String),
}

/// Result type for MCP operations
pub type McpResult<T> = Result<T, McpError>;
