// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Smelt MCP Server
//!
//! This binary implements the Model Context Protocol (MCP) server for Smelt,
//! allowing AI assistants to interact with Smelt's semantic version control
//! capabilities programmatically.
//!
//! The server communicates over stdin/stdout using JSON-RPC 2.0.

use anyhow::Result;
use smelt_mcp::{
    context::SmeltContext,
    protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse},
    server::McpServer,
};
use std::io::{self, BufRead, Write};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

fn main() -> Result<()> {
    // Initialize logging to stderr (so it doesn't interfere with JSON-RPC)
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(io::stderr)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Smelt MCP server v{}", env!("CARGO_PKG_VERSION"));

    // Create tokio runtime
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async { run_server().await })
}

async fn run_server() -> Result<()> {
    let context = SmeltContext::new();
    let mut server = McpServer::new(context);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Process JSON-RPC messages line by line
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to read stdin: {}", e);
                break;
            }
        };

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        tracing::debug!("Received: {}", &line);

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to parse request: {}", e);
                let error_response = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    JsonRpcError::parse_error(e.to_string()),
                );
                write_response(&mut stdout, &error_response)?;
                continue;
            }
        };

        tracing::info!("Handling method: {}", request.method);

        // Handle request
        let response = server.handle_request(request).await;

        // Send response
        write_response(&mut stdout, &response)?;
    }

    tracing::info!("Smelt MCP server shutting down");
    Ok(())
}

fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<()> {
    let response_json = serde_json::to_string(response)?;
    tracing::debug!("Sending: {}", &response_json);
    writeln!(stdout, "{}", response_json)?;
    stdout.flush()?;
    Ok(())
}
