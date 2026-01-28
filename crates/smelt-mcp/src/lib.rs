//! Smelt MCP Server
//!
//! This crate provides a Model Context Protocol (MCP) server for Smelt,
//! enabling AI assistants to interact with Smelt's semantic version control
//! capabilities programmatically.
//!
//! ## Features
//!
//! - **Intent Management**: Create and track development intents
//! - **Status & Validation**: Check semantic status and validate changes
//! - **Commits**: Create commits with semantic deltas attached
//! - **Episodic Memory**: Search, capture, and learn from past experiences
//!
//! ## Protocol
//!
//! The server communicates over stdio using JSON-RPC 2.0 with newline-delimited
//! messages.
//!
//! ## Tools
//!
//! - `smelt_init` - Initialize Smelt in a repository
//! - `smelt_status` - Show semantic status
//! - `smelt_validate` - Validate changes against constraints
//! - `smelt_commit` - Commit with semantic delta
//! - `smelt_intent_create` - Create a new intent
//! - `smelt_intent_list` - List intents
//! - `smelt_memory_search` - Search episodic memory
//! - `smelt_memory_capture` - Capture an episode
//! - `smelt_memory_feedback` - Provide feedback on episodes

pub mod context;
pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;

pub use context::SmeltContext;
pub use error::{McpError, McpResult};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, Tool, ToolResult};
pub use server::McpServer;
