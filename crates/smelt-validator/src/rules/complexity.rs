// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Complexity threshold checking

use super::ValidationRule;
use crate::config::ComplexityConfig;
use crate::validator::{ValidationSeverity, Violation};
use smelt_core::{IntentRecord, SemanticDelta};

/// Checks for complexity threshold violations
pub struct ComplexityChecker {
    config: ComplexityConfig,
}

impl ComplexityChecker {
    /// Create a new complexity checker
    pub fn new(config: ComplexityConfig) -> Self {
        Self { config }
    }
}

impl ValidationRule for ComplexityChecker {
    fn name(&self) -> &'static str {
        "complexity"
    }

    fn validate(&self, delta: &SemanticDelta, _intent: Option<&IntentRecord>) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Check overall complexity increase
        let complexity_delta = delta.impact_summary.complexity_delta;
        if complexity_delta > self.config.max_complexity_increase {
            violations.push(Violation {
                rule: "complexity-increase".to_string(),
                severity: if self.config.complexity_error {
                    ValidationSeverity::Error
                } else {
                    ValidationSeverity::Warning
                },
                message: format!(
                    "Complexity increased by {} (threshold: {})",
                    complexity_delta, self.config.max_complexity_increase
                ),
                location: None,
                suggestion: Some(
                    "Consider refactoring to reduce complexity or splitting into smaller functions"
                        .to_string(),
                ),
            });
        }

        // Check individual function complexity from changes
        // Note: Full complexity analysis would require AST access
        // This checks complexity_delta in BodyModified changes
        for change in &delta.changes {
            if let smelt_core::SemanticChange::BodyModified {
                name,
                file,
                complexity_delta: func_delta,
            } = change
            {
                if *func_delta > self.config.max_complexity_increase {
                    violations.push(Violation {
                        rule: "function-complexity".to_string(),
                        severity: if self.config.complexity_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!(
                            "Function '{}' complexity increased by {}",
                            name, func_delta
                        ),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Extract helper functions or simplify control flow".to_string(),
                        ),
                    });
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smelt_core::{ImpactSummary, SemanticChange};
    use uuid::Uuid;

    fn make_delta_with_complexity(complexity_delta: i32) -> SemanticDelta {
        SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            from_snapshot: Uuid::new_v4(),
            to_snapshot: Uuid::new_v4(),
            changes: Vec::new(),
            impact_summary: ImpactSummary {
                complexity_delta,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_complexity_under_threshold() {
        let checker = ComplexityChecker::new(ComplexityConfig::default());

        let delta = make_delta_with_complexity(5);
        let violations = checker.validate(&delta, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_complexity_over_threshold() {
        let checker = ComplexityChecker::new(ComplexityConfig::default());

        let delta = make_delta_with_complexity(15);
        let violations = checker.validate(&delta, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "complexity-increase");
    }

    #[test]
    fn test_function_complexity_change() {
        let checker = ComplexityChecker::new(ComplexityConfig::default());

        let delta = SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            from_snapshot: Uuid::new_v4(),
            to_snapshot: Uuid::new_v4(),
            changes: vec![SemanticChange::BodyModified {
                name: "complex_function".to_string(),
                file: "lib.rs".to_string(),
                complexity_delta: 15,
            }],
            impact_summary: ImpactSummary::default(),
        };

        let violations = checker.validate(&delta, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "function-complexity");
    }
}
