//! Smelt API Server - REST API for AI agent integration

use anyhow::Result;
use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod handlers;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::new("info"))
        .init();

    // Get working directory (can be overridden via env)
    let work_dir = std::env::var("SMELT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    let smelt_dir = work_dir.join(".smelt");
    if !smelt_dir.exists() {
        tracing::error!("Smelt not initialized in {:?}", work_dir);
        tracing::error!("Run 'smelt init' first, then start the API server.");
        std::process::exit(1);
    }

    // Create shared state (just paths, connections created per-request)
    let state = AppState::new(&smelt_dir);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Intent endpoints
        .route("/api/v1/intents", get(handlers::list_intents))
        .route("/api/v1/intents", post(handlers::create_intent))
        .route("/api/v1/intents/:id", get(handlers::get_intent))
        // Delta endpoints
        .route("/api/v1/deltas/:id", get(handlers::get_delta))
        .route("/api/v1/intents/:id/delta", get(handlers::get_intent_delta))
        // Validation endpoints
        .route("/api/v1/validate", post(handlers::validate))
        // Memory endpoints
        .route("/api/v1/memory/search", post(handlers::memory_search))
        .route("/api/v1/memory/episodes/:id", get(handlers::get_episode))
        .route(
            "/api/v1/memory/episodes/:id/feedback",
            post(handlers::episode_feedback),
        )
        // Status endpoint
        .route("/api/v1/status", get(handlers::status))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let port: u16 = std::env::var("SMELT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Smelt API server listening on http://{}", addr);
    tracing::info!("Working directory: {:?}", work_dir);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
