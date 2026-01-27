//! Breaking change detection

use super::ValidationRule;
use crate::config::SemanticConfig;
use crate::validator::{ValidationSeverity, Violation};
use smelt_core::{IntentRecord, SemanticChange, SemanticDelta};

/// Checks for breaking changes in semantic deltas
pub struct BreakingChangeChecker {
    config: SemanticConfig,
}

impl BreakingChangeChecker {
    /// Create a new breaking change checker
    pub fn new(config: SemanticConfig) -> Self {
        Self { config }
    }

    /// Check if a change is breaking
    fn is_breaking_change(&self, change: &SemanticChange) -> Option<Violation> {
        match change {
            SemanticChange::FunctionRemoved {
                name,
                file,
                was_public,
            } => {
                if *was_public {
                    Some(Violation {
                        rule: "breaking-change".to_string(),
                        severity: if self.config.breaking_changes_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!("Public function '{}' was removed", name),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Mark as deprecated instead of removing, or ensure no external consumers".to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            SemanticChange::SignatureChanged {
                name,
                file,
                old_signature,
                new_signature,
                is_breaking,
            } => {
                if *is_breaking {
                    Some(Violation {
                        rule: "breaking-change".to_string(),
                        severity: if self.config.breaking_changes_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!(
                            "Breaking signature change in '{}': '{}' -> '{}'",
                            name, old_signature, new_signature
                        ),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Add overload or default parameter to maintain backward compatibility"
                                .to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            SemanticChange::TypeRemoved {
                name,
                file,
                was_public,
            } => {
                if *was_public {
                    Some(Violation {
                        rule: "breaking-change".to_string(),
                        severity: if self.config.breaking_changes_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!("Public type '{}' was removed", name),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Mark as deprecated instead of removing, or provide migration path"
                                .to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            SemanticChange::TypeModified {
                name,
                file,
                fields_removed,
                is_breaking,
                ..
            } => {
                if *is_breaking && !fields_removed.is_empty() {
                    Some(Violation {
                        rule: "breaking-change".to_string(),
                        severity: if self.config.breaking_changes_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!(
                            "Breaking change in type '{}': fields removed: {}",
                            name,
                            fields_removed.join(", ")
                        ),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Mark fields as deprecated instead of removing".to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            SemanticChange::VisibilityChanged {
                name,
                file,
                old_visibility,
                new_visibility,
            } => {
                use smelt_core::Visibility;
                // Reducing visibility is a breaking change
                let is_reduction = matches!(
                    (old_visibility, new_visibility),
                    (Visibility::Public, Visibility::Internal)
                        | (Visibility::Public, Visibility::Private)
                        | (Visibility::Internal, Visibility::Private)
                );

                if is_reduction {
                    Some(Violation {
                        rule: "breaking-change".to_string(),
                        severity: if self.config.breaking_changes_error {
                            ValidationSeverity::Error
                        } else {
                            ValidationSeverity::Warning
                        },
                        message: format!(
                            "Visibility reduced for '{}': {:?} -> {:?}",
                            name, old_visibility, new_visibility
                        ),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Ensure no external consumers before reducing visibility".to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            _ => None,
        }
    }
}

impl ValidationRule for BreakingChangeChecker {
    fn name(&self) -> &'static str {
        "breaking-changes"
    }

    fn validate(&self, delta: &SemanticDelta, _intent: Option<&IntentRecord>) -> Vec<Violation> {
        if !self.config.check_breaking_changes {
            return Vec::new();
        }

        delta
            .changes
            .iter()
            .filter_map(|change| self.is_breaking_change(change))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smelt_core::{ImpactSummary, Visibility};
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
    fn test_public_function_removal() {
        let checker = BreakingChangeChecker::new(SemanticConfig::default());

        let delta = make_delta(vec![SemanticChange::FunctionRemoved {
            name: "process".to_string(),
            file: "lib.rs".to_string(),
            was_public: true,
        }]);

        let violations = checker.validate(&delta, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "breaking-change");
    }

    #[test]
    fn test_private_function_removal_ok() {
        let checker = BreakingChangeChecker::new(SemanticConfig::default());

        let delta = make_delta(vec![SemanticChange::FunctionRemoved {
            name: "helper".to_string(),
            file: "lib.rs".to_string(),
            was_public: false,
        }]);

        let violations = checker.validate(&delta, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_visibility_reduction() {
        let checker = BreakingChangeChecker::new(SemanticConfig::default());

        let delta = make_delta(vec![SemanticChange::VisibilityChanged {
            name: "MyStruct".to_string(),
            file: "lib.rs".to_string(),
            old_visibility: Visibility::Public,
            new_visibility: Visibility::Private,
        }]);

        let violations = checker.validate(&delta, None);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_disabled_check() {
        let config = SemanticConfig {
            check_breaking_changes: false,
            ..Default::default()
        };

        let checker = BreakingChangeChecker::new(config);

        let delta = make_delta(vec![SemanticChange::FunctionRemoved {
            name: "process".to_string(),
            file: "lib.rs".to_string(),
            was_public: true,
        }]);

        let violations = checker.validate(&delta, None);
        assert!(violations.is_empty());
    }
}
