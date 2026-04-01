// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Error types for smelt-memory

use thiserror::Error;

/// Memory-specific errors
#[derive(Error, Debug)]
pub enum MemoryError {
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// SQLite error
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Embedding error
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Episode not found
    #[error("Episode not found: {0}")]
    NotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Result type for memory operations
pub type MemoryResult<T> = Result<T, MemoryError>;
