// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Intent validation - validate delta matches intent constraints

use crate::config::IntentConfig;
use crate::rules::ValidationRule;
use crate::validator::{ValidationSeverity, Violation};
use smelt_core::{IntentRecord, SemanticDelta};

/// Validates that semantic deltas match intent constraints
pub struct IntentValidator {
    config: IntentConfig,
}

impl IntentValidator {
    /// Create a new intent validator
    pub fn new(config: IntentConfig) -> Self {
        Self { config }
    }

    /// Check if the delta scope is reasonable for the intent
    fn check_scope(&self, delta: &SemanticDelta, intent: &IntentRecord) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Check if change is too large without rationale
        if self.config.require_rationale_for_large_changes
            && delta.impact_summary.files_affected >= self.config.large_change_threshold
            && intent.rationale.is_none()
        {
            violations.push(Violation {
                rule: "large-change-rationale".to_string(),
                severity: ValidationSeverity::Warning,
                message: format!(
                    "Large change ({} files affected) without rationale",
                    delta.impact_summary.files_affected
                ),
                location: None,
                suggestion: Some(format!(
                    "Provide rationale for changes affecting {} or more files",
                    self.config.large_change_threshold
                )),
            });
        }

        // Check intent constraints
        for constraint in &intent.constraints {
            if constraint.required {
                // TODO: Implement constraint validation based on constraint type
                // For now, just acknowledge the constraint exists
                tracing::debug!(
                    "Checking constraint: {} = {}",
                    constraint.name,
                    constraint.value
                );
            }
        }

        violations
    }

    /// Check if breaking changes are allowed by intent
    fn check_breaking_allowed(
        &self,
        delta: &SemanticDelta,
        intent: &IntentRecord,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Check if intent explicitly allows breaking changes
        let breaking_allowed = intent
            .constraints
            .iter()
            .any(|c| c.name == "allow_breaking_changes" && c.value.to_lowercase() == "true");

        if delta.impact_summary.breaking_changes > 0 && !breaking_allowed {
            violations.push(Violation {
                rule: "breaking-not-allowed".to_string(),
                severity: ValidationSeverity::Error,
                message: format!(
                    "{} breaking change(s) detected but intent does not allow breaking changes",
                    delta.impact_summary.breaking_changes
                ),
                location: None,
                suggestion: Some(
                    "Add constraint 'allow_breaking_changes: true' to intent if intentional"
                        .to_string(),
                ),
            });
        }

        violations
    }
}

impl ValidationRule for IntentValidator {
    fn name(&self) -> &'static str {
        "intent"
    }

    fn validate(&self, delta: &SemanticDelta, intent: Option<&IntentRecord>) -> Vec<Violation> {
        let Some(intent) = intent else {
            return Vec::new();
        };

        if !self.config.validate_scope {
            return Vec::new();
        }

        let mut violations = Vec::new();
        violations.extend(self.check_scope(delta, intent));
        violations.extend(self.check_breaking_allowed(delta, intent));

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smelt_core::{Author, AuthorType, Constraint, ContextLinks, ImpactSummary, IntentStatus};
    use uuid::Uuid;

    fn make_intent(rationale: Option<String>, constraints: Vec<Constraint>) -> IntentRecord {
        IntentRecord {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            author: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
                author_type: AuthorType::Human,
            },
            goal: "Test goal".to_string(),
            rationale,
            constraints,
            context_links: ContextLinks::default(),
            status: IntentStatus::InProgress,
            baseline_snapshot_id: None,
        }
    }

    fn make_delta(files_affected: usize, breaking_changes: usize) -> SemanticDelta {
        SemanticDelta {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            from_snapshot: Uuid::new_v4(),
            to_snapshot: Uuid::new_v4(),
            changes: Vec::new(),
            impact_summary: ImpactSummary {
                files_affected,
                breaking_changes,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_large_change_with_rationale_ok() {
        let config = IntentConfig {
            require_rationale_for_large_changes: true,
            large_change_threshold: 5,
            ..Default::default()
        };

        let validator = IntentValidator::new(config);
        let intent = make_intent(Some("Major refactoring".to_string()), vec![]);
        let delta = make_delta(10, 0);

        let violations = validator.validate(&delta, Some(&intent));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_large_change_without_rationale() {
        let config = IntentConfig {
            require_rationale_for_large_changes: true,
            large_change_threshold: 5,
            ..Default::default()
        };

        let validator = IntentValidator::new(config);
        let intent = make_intent(None, vec![]);
        let delta = make_delta(10, 0);

        let violations = validator.validate(&delta, Some(&intent));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "large-change-rationale");
    }

    #[test]
    fn test_breaking_changes_not_allowed() {
        let config = IntentConfig::default();

        let validator = IntentValidator::new(config);
        let intent = make_intent(None, vec![]);
        let delta = make_delta(1, 2);

        let violations = validator.validate(&delta, Some(&intent));
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "breaking-not-allowed");
    }

    #[test]
    fn test_breaking_changes_allowed_by_constraint() {
        let config = IntentConfig::default();

        let validator = IntentValidator::new(config);
        let intent = make_intent(
            None,
            vec![Constraint {
                name: "allow_breaking_changes".to_string(),
                value: "true".to_string(),
                required: false,
            }],
        );
        let delta = make_delta(1, 2);

        let violations = validator.validate(&delta, Some(&intent));
        assert!(violations.is_empty());
    }
}
