// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Error types for Smelt validator

use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = std::result::Result<T, ValidationError>;

/// Validation error types
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Rule parsing error
    #[error("Rule parsing error: {0}")]
    RuleParsing(String),

    /// Crucible integration error
    #[error("Crucible error: {0}")]
    Crucible(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Smelt core error
    #[error("Smelt error: {0}")]
    Smelt(#[from] smelt_core::SmeltError),
}
