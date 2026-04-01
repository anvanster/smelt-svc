// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Validation configuration

use crate::error::ValidationResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Smelt validation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Architectural validation rules
    #[serde(default)]
    pub architecture: ArchitectureConfig,

    /// Semantic delta validation rules
    #[serde(default)]
    pub semantic: SemanticConfig,

    /// Intent validation rules
    #[serde(default)]
    pub intent: IntentConfig,
}

/// Architectural validation configuration (Crucible-compatible)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchitectureConfig {
    /// Layer definitions
    #[serde(default)]
    pub layers: Vec<Layer>,

    /// Whether to check for circular dependencies
    #[serde(default = "default_true")]
    pub check_circular_deps: bool,

    /// Whether to enforce layer boundaries
    #[serde(default = "default_true")]
    pub enforce_layers: bool,
}

/// Layer definition for architectural boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Layer name
    pub name: String,

    /// Path patterns that belong to this layer
    pub paths: Vec<String>,

    /// Layers this layer can depend on
    #[serde(default)]
    pub can_depend_on: Vec<String>,

    /// Layers this layer cannot depend on
    #[serde(default)]
    pub prohibited_dependencies: Vec<String>,
}

/// Semantic delta validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    /// Whether to check for breaking changes
    #[serde(default = "default_true")]
    pub check_breaking_changes: bool,

    /// Whether breaking changes are errors or warnings
    #[serde(default = "default_true")]
    pub breaking_changes_error: bool,

    /// Whether to check visibility changes
    #[serde(default = "default_true")]
    pub check_visibility: bool,

    /// Whether new public API needs review
    #[serde(default)]
    pub review_new_public_api: bool,

    /// Complexity thresholds
    #[serde(default)]
    pub complexity: ComplexityConfig,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            check_breaking_changes: true,
            breaking_changes_error: true,
            check_visibility: true,
            review_new_public_api: false,
            complexity: ComplexityConfig::default(),
        }
    }
}

/// Complexity validation thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityConfig {
    /// Maximum allowed cyclomatic complexity per function
    #[serde(default = "default_complexity")]
    pub max_cyclomatic: u32,

    /// Maximum allowed cognitive complexity per function
    #[serde(default = "default_cognitive")]
    pub max_cognitive: u32,

    /// Maximum allowed complexity increase in a single delta
    #[serde(default = "default_delta")]
    pub max_complexity_increase: i32,

    /// Whether complexity violations are errors
    #[serde(default)]
    pub complexity_error: bool,
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            max_cyclomatic: 15,
            max_cognitive: 25,
            max_complexity_increase: 10,
            complexity_error: false,
        }
    }
}

/// Intent validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentConfig {
    /// Whether to validate delta matches intent
    #[serde(default = "default_true")]
    pub validate_scope: bool,

    /// Whether to require rationale for large changes
    #[serde(default)]
    pub require_rationale_for_large_changes: bool,

    /// Threshold for "large" changes (file count)
    #[serde(default = "default_large_threshold")]
    pub large_change_threshold: usize,
}

impl Default for IntentConfig {
    fn default() -> Self {
        Self {
            validate_scope: true,
            require_rationale_for_large_changes: false,
            large_change_threshold: 10,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_complexity() -> u32 {
    15
}

fn default_cognitive() -> u32 {
    25
}

fn default_delta() -> i32 {
    10
}

fn default_large_threshold() -> usize {
    10
}

impl ValidationConfig {
    /// Load configuration from a YAML file
    pub fn load(path: &Path) -> ValidationResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from smelt directory, with fallback to defaults
    pub fn load_or_default(smelt_dir: &Path) -> Self {
        // Try crucible.yaml first
        let crucible_path = smelt_dir.join("crucible.yaml");
        if crucible_path.exists() {
            if let Ok(config) = Self::load(&crucible_path) {
                return config;
            }
        }

        // Try smelt validation config
        let validation_path = smelt_dir.join("validation.yaml");
        if validation_path.exists() {
            if let Ok(config) = Self::load(&validation_path) {
                return config;
            }
        }

        // Return defaults
        Self::default()
    }

    /// Create a strict configuration for production use
    pub fn strict() -> Self {
        Self {
            architecture: ArchitectureConfig {
                check_circular_deps: true,
                enforce_layers: true,
                layers: Vec::new(),
            },
            semantic: SemanticConfig {
                check_breaking_changes: true,
                breaking_changes_error: true,
                check_visibility: true,
                review_new_public_api: true,
                complexity: ComplexityConfig {
                    max_cyclomatic: 10,
                    max_cognitive: 15,
                    max_complexity_increase: 5,
                    complexity_error: true,
                },
            },
            intent: IntentConfig {
                validate_scope: true,
                require_rationale_for_large_changes: true,
                large_change_threshold: 5,
            },
        }
    }
}
