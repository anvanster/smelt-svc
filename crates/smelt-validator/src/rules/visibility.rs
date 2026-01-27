//! Visibility change detection

use super::ValidationRule;
use crate::config::SemanticConfig;
use crate::validator::{ValidationSeverity, Violation};
use smelt_core::{IntentRecord, SemanticChange, SemanticDelta, Visibility};

/// Checks for visibility changes and new public API
pub struct VisibilityChecker {
    config: SemanticConfig,
}

impl VisibilityChecker {
    /// Create a new visibility checker
    pub fn new(config: SemanticConfig) -> Self {
        Self { config }
    }

    /// Check for new public API additions
    fn check_new_public_api(&self, change: &SemanticChange) -> Option<Violation> {
        match change {
            SemanticChange::FunctionAdded {
                name,
                file,
                is_public,
                ..
            } => {
                if *is_public && self.config.review_new_public_api {
                    Some(Violation {
                        rule: "new-public-api".to_string(),
                        severity: ValidationSeverity::Warning,
                        message: format!("New public function '{}' added", name),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Ensure API design is reviewed before committing public interface"
                                .to_string(),
                        ),
                    })
                } else {
                    None
                }
            }

            SemanticChange::TypeAdded {
                name,
                file,
                is_public,
                kind,
            } => {
                if *is_public && self.config.review_new_public_api {
                    Some(Violation {
                        rule: "new-public-api".to_string(),
                        severity: ValidationSeverity::Warning,
                        message: format!("New public {} '{}' added", kind, name),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Ensure type design is reviewed before committing public interface"
                                .to_string(),
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
                // Check for visibility increase (internal -> public)
                let is_increase = matches!(
                    (old_visibility, new_visibility),
                    (Visibility::Private, Visibility::Internal)
                        | (Visibility::Private, Visibility::Public)
                        | (Visibility::Internal, Visibility::Public)
                );

                if is_increase
                    && *new_visibility == Visibility::Public
                    && self.config.review_new_public_api
                {
                    Some(Violation {
                        rule: "new-public-api".to_string(),
                        severity: ValidationSeverity::Warning,
                        message: format!(
                            "'{}' visibility increased to public ({:?} -> {:?})",
                            name, old_visibility, new_visibility
                        ),
                        location: Some(file.clone()),
                        suggestion: Some(
                            "Ensure API is ready for public consumption".to_string(),
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

impl ValidationRule for VisibilityChecker {
    fn name(&self) -> &'static str {
        "visibility"
    }

    fn validate(
        &self,
        delta: &SemanticDelta,
        _intent: Option<&IntentRecord>,
    ) -> Vec<Violation> {
        if !self.config.check_visibility {
            return Vec::new();
        }

        delta
            .changes
            .iter()
            .filter_map(|change| self.check_new_public_api(change))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smelt_core::ImpactSummary;
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
    fn test_new_public_function_flagged() {
        let mut config = SemanticConfig::default();
        config.review_new_public_api = true;

        let checker = VisibilityChecker::new(config);

        let delta = make_delta(vec![SemanticChange::FunctionAdded {
            name: "new_api".to_string(),
            file: "lib.rs".to_string(),
            signature: "fn new_api()".to_string(),
            is_public: true,
        }]);

        let violations = checker.validate(&delta, None);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, "new-public-api");
    }

    #[test]
    fn test_private_function_ok() {
        let mut config = SemanticConfig::default();
        config.review_new_public_api = true;

        let checker = VisibilityChecker::new(config);

        let delta = make_delta(vec![SemanticChange::FunctionAdded {
            name: "helper".to_string(),
            file: "lib.rs".to_string(),
            signature: "fn helper()".to_string(),
            is_public: false,
        }]);

        let violations = checker.validate(&delta, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_review_disabled_no_warnings() {
        let mut config = SemanticConfig::default();
        config.review_new_public_api = false;

        let checker = VisibilityChecker::new(config);

        let delta = make_delta(vec![SemanticChange::FunctionAdded {
            name: "new_api".to_string(),
            file: "lib.rs".to_string(),
            signature: "fn new_api()".to_string(),
            is_public: true,
        }]);

        let violations = checker.validate(&delta, None);
        assert!(violations.is_empty());
    }
}
