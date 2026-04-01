// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Main SmeltValidator implementation

use crate::config::ValidationConfig;
use crate::crucible::CrucibleAdapter;
use crate::rules::{BreakingChangeChecker, ComplexityChecker, ValidationRule, VisibilityChecker};
use crate::semantic::IntentValidator;
use serde::{Deserialize, Serialize};
use smelt_core::{IntentRecord, SemanticDelta};
use std::path::Path;

/// Severity of a validation violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Informational only
    Info,
    /// Warning - may proceed
    Warning,
    /// Error - should not proceed
    Error,
}

/// A validation violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Rule that was violated
    pub rule: String,
    /// Severity of the violation
    pub severity: ValidationSeverity,
    /// Human-readable message
    pub message: String,
    /// Location in code (file path)
    pub location: Option<String>,
    /// Suggestion for fixing
    pub suggestion: Option<String>,
}

/// Overall validation outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutcome {
    /// Whether validation passed (no errors)
    pub passed: bool,
    /// List of violations
    pub violations: Vec<Violation>,
    /// Count of errors
    pub error_count: usize,
    /// Count of warnings
    pub warning_count: usize,
    /// Count of info messages
    pub info_count: usize,
}

impl ValidationOutcome {
    /// Create a passing outcome
    pub fn pass() -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Get all error violations
    pub fn errors(&self) -> impl Iterator<Item = &Violation> {
        self.violations
            .iter()
            .filter(|v| v.severity == ValidationSeverity::Error)
    }

    /// Get all warning violations
    pub fn warnings(&self) -> impl Iterator<Item = &Violation> {
        self.violations
            .iter()
            .filter(|v| v.severity == ValidationSeverity::Warning)
    }
}

/// Main validator for Smelt
pub struct SmeltValidator {
    config: ValidationConfig,
    rules: Vec<Box<dyn ValidationRule>>,
}

impl SmeltValidator {
    /// Create a new validator with the given configuration
    pub fn new(config: ValidationConfig) -> Self {
        // Add semantic delta validators
        let rules: Vec<Box<dyn ValidationRule>> = vec![
            Box::new(BreakingChangeChecker::new(config.semantic.clone())),
            Box::new(VisibilityChecker::new(config.semantic.clone())),
            Box::new(ComplexityChecker::new(config.semantic.complexity.clone())),
            Box::new(IntentValidator::new(config.intent.clone())),
        ];

        Self { config, rules }
    }

    /// Create a validator with default configuration
    pub fn default_config() -> Self {
        Self::new(ValidationConfig::default())
    }

    /// Create a validator with strict configuration
    pub fn strict() -> Self {
        Self::new(ValidationConfig::strict())
    }

    /// Load a validator from a smelt directory
    pub fn from_smelt_dir(smelt_dir: &Path) -> Self {
        let config = ValidationConfig::load_or_default(smelt_dir);
        let mut validator = Self::new(config);

        // Add Crucible adapter if architecture config enables it
        if validator.config.architecture.check_circular_deps
            || validator.config.architecture.enforce_layers
        {
            // Get project root (parent of .smelt)
            if let Some(project_root) = smelt_dir.parent() {
                let crucible = CrucibleAdapter::new(project_root)
                    .with_circular_deps(validator.config.architecture.check_circular_deps);
                validator.add_rule(Box::new(crucible));
            }
        }

        validator
    }

    /// Get the configuration
    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }

    /// Validate a semantic delta
    pub fn validate(
        &self,
        delta: &SemanticDelta,
        intent: Option<&IntentRecord>,
    ) -> ValidationOutcome {
        let mut violations = Vec::new();

        // Run all validation rules
        for rule in &self.rules {
            let rule_violations = rule.validate(delta, intent);
            violations.extend(rule_violations);
        }

        // Count by severity
        let error_count = violations
            .iter()
            .filter(|v| v.severity == ValidationSeverity::Error)
            .count();
        let warning_count = violations
            .iter()
            .filter(|v| v.severity == ValidationSeverity::Warning)
            .count();
        let info_count = violations
            .iter()
            .filter(|v| v.severity == ValidationSeverity::Info)
            .count();

        ValidationOutcome {
            passed: error_count == 0,
            violations,
            error_count,
            warning_count,
            info_count,
        }
    }

    /// Validate a delta and return a simple pass/fail result
    pub fn validate_simple(&self, delta: &SemanticDelta, intent: Option<&IntentRecord>) -> bool {
        self.validate(delta, intent).passed
    }

    /// Add a custom validation rule
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }
}

impl Default for SmeltValidator {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smelt_core::{ImpactSummary, SemanticChange};
    use uuid::Uuid;

    fn make_delta(changes: Vec<SemanticChange>) -> SemanticDelta {
        SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            from_snapshot: Uuid::new_v4(),
            to_snapshot: Uuid::new_v4(),
            changes,
            impact_summary: ImpactSummary::default(),
        }
    }

    #[test]
    fn test_empty_delta_passes() {
        let validator = SmeltValidator::default_config();
        let delta = make_delta(vec![]);

        let outcome = validator.validate(&delta, None);
        assert!(outcome.passed);
        assert_eq!(outcome.error_count, 0);
    }

    #[test]
    fn test_breaking_change_fails() {
        let validator = SmeltValidator::default_config();

        let delta = make_delta(vec![SemanticChange::FunctionRemoved {
            name: "public_api".to_string(),
            file: "lib.rs".to_string(),
            was_public: true,
        }]);

        let outcome = validator.validate(&delta, None);
        assert!(!outcome.passed);
        assert_eq!(outcome.error_count, 1);
    }

    #[test]
    fn test_private_removal_passes() {
        let validator = SmeltValidator::default_config();

        let delta = make_delta(vec![SemanticChange::FunctionRemoved {
            name: "helper".to_string(),
            file: "lib.rs".to_string(),
            was_public: false,
        }]);

        let outcome = validator.validate(&delta, None);
        assert!(outcome.passed);
    }

    #[test]
    fn test_strict_validator() {
        let validator = SmeltValidator::strict();

        // Even smaller complexity increase should fail in strict mode
        let delta = SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            from_snapshot: Uuid::new_v4(),
            to_snapshot: Uuid::new_v4(),
            changes: vec![],
            impact_summary: ImpactSummary {
                complexity_delta: 10, // Over strict threshold of 5
                ..Default::default()
            },
        };

        let outcome = validator.validate(&delta, None);
        // Strict mode makes complexity errors
        assert!(outcome.has_errors() || outcome.has_warnings());
    }

    #[test]
    fn test_outcome_helpers() {
        let outcome = ValidationOutcome {
            passed: false,
            violations: vec![
                Violation {
                    rule: "test".to_string(),
                    severity: ValidationSeverity::Error,
                    message: "error".to_string(),
                    location: None,
                    suggestion: None,
                },
                Violation {
                    rule: "test".to_string(),
                    severity: ValidationSeverity::Warning,
                    message: "warning".to_string(),
                    location: None,
                    suggestion: None,
                },
            ],
            error_count: 1,
            warning_count: 1,
            info_count: 0,
        };

        assert!(outcome.has_errors());
        assert!(outcome.has_warnings());
        assert_eq!(outcome.errors().count(), 1);
        assert_eq!(outcome.warnings().count(), 1);
    }
}
